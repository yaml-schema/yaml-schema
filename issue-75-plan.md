# Support multiple positional input files in `ys` CLI (issue #75)

## Context

`ys` currently accepts exactly one positional `FILE` argument (`Option<String>`). Passing more than one (e.g. via shell globs, or batching in a `pre-commit` hook) fails with `error: unexpected argument 'file2.yml' found`. Issue #75 (yaml-schema/yaml-schema#75) asks for `ys [OPTIONS] [FILES...]`: accept multiple files, load/compile the schema once, and validate each file against it, matching standard UNIX tool conventions and enabling native `pre-commit` support (see #71).

Decisions confirmed with the user:
- **Reporting**: annotate each result with its source filename — a `"file"` field on every JSON error entry, and a filename header line in human-readable output before each file's errors (only emitted when more than one file is given, so single-file output is byte-for-byte unchanged).
- **`--fail-fast` scope**: extend it to also stop the whole multi-file run as soon as any file produces an error (in addition to its existing within-file short-circuit behavior).
- **Parallelism**: out of scope. No `rayon` dependency added — the loop is sequential. The stated perf win (schema read/compiled once) is achieved regardless of file-loop parallelism.

## Implementation

All changes are in `src/bin/ys.rs`.

### 1. CLI struct

```rust
/// The YAML file(s) to validate
#[arg(value_name = "FILES")]
pub files: Vec<String>,
```

Replaces `pub file: Option<String>` (currently line 41).

### 2. `command_validate` restructuring

Current logic (lines 123–244) does: read one file → branch on `-f` present/absent to resolve one `(root_schema, preloaded)` pair → call `Engine::evaluate_with_schemas` once → print/return.

New logic:

1. If `opts.files.is_empty()`, return `Err(eyre::eyre!("No YAML files specified"))` (same shape as today, pluralized).
2. **Schema resolution stays split on `-f` presence, but only the `-f` branch can be resolved once, outside the loop:**
   - If `opts.schemas` is non-empty: run the existing root-schema + preload-map loading (current lines 133–177) **once**, before the file loop. Reuse the resulting `Rc<RootSchema>` and `HashMap<String, Rc<RootSchema>>` for every file (the map is cloned per call since `evaluate_with_schemas` takes it by value — cheap, since values are `Rc`).
   - If `opts.schemas` is empty: the `$schema`-in-instance extraction (current lines 178–213) is inherently per-file (each file's parent dir and each file's own `$schema:` value can differ), so it moves **inside** the loop, run fresh for each file.
3. Loop over `opts.files`, tracking `any_errors: bool` and (for `--json`) an accumulated `Vec<serde_json::Value>` of error entries across all files:
   - Read the file's contents (`std::fs::read_to_string`). On failure, treat as a file-level error (see below), continue to the next file (or stop, if `--fail-fast`).
   - Resolve `(root_for_eval, preloaded)` for this file per step 2. On resolution failure, same handling as today (`emit_json_error`/`eprintln!` + treat as failing file) rather than aborting the whole run — except propagate a hard `Err` only for the "no schema found" case when there is exactly one file, to preserve today's exact error message/behavior for the single-file no-`$schema` case. When there are multiple files, a missing `$schema` on one file should be reported as a per-file failure so the rest of the batch still runs.
   - Call `Engine::evaluate_with_schemas(root_for_eval.as_ref(), &yaml_contents, opts.fail_fast, preloaded)`.
   - On `Ok(context)` with `context.has_errors()`: mark `any_errors = true`.
     - Human mode: if `opts.files.len() > 1`, print a header line with the filename before this file's errors (something like `eprintln!("{file}:")`), then the existing per-error `eprintln!("{error}")` loop, unchanged.
     - JSON mode: build each entry as today (`emit_validation_errors_json`'s per-entry shape) but add a `"file": file` key, and push into the accumulated `Vec` instead of printing immediately.
   - On `Err(e)` from `Engine::evaluate_with_schemas`, or a file read/schema-resolution failure: mark `any_errors = true` and report it as a file-level failure (human: `eprintln!("{file}: {e}")` or similar; JSON: push `{"file": file, "error": e.to_string()}` into the accumulated array — no `path`/`line`/`col` for this case, consistent with existing tests only ever asserting `path`/`error` are *present*, not an exhaustive key set).
   - If `opts.fail_fast && any_errors`: `break` out of the file loop immediately.
4. After the loop: in JSON mode, print the single accumulated array (`println!("{}", serde_json::Value::Array(all_entries))`) only if non-empty — same shape/behavior as today for the single-file case. Return `Ok(1)` if `any_errors`, else `Ok(0)`.

Refactor `emit_validation_errors_json` to build+return `Vec<serde_json::Value>` (with the new `"file"` key) instead of printing directly, so the loop can accumulate across files and the caller does one final `println!`.

### 3. Tests

- `features/cli.feature`: add scenarios for multiple files, e.g.:
  - Multiple valid files with `-f` → exit 0.
  - Mix of valid/invalid files with `-f` → exit 1, stderr shows a header per file and each file's errors.
  - Multiple files with `--json` → stdout is one JSON array whose entries carry the correct `"file"` values.
  - `--fail-fast` with multiple files, first file invalid → only the first file's errors appear (second file never validated).
  - Existing single-file scenarios should keep passing unchanged (no header line, same JSON shape) — a spot check, not a rewrite.
- `tests/ys_cli_json.rs`: add a case invoking `ys` with two instance files and asserting the returned array's entries include a `"file"` key matching each temp file path, using the same `tempdir()`/`assert_cmd::Command::cargo_bin("ys")` pattern as the existing tests.
- Reuse existing fixtures under `tests/fixtures/` (`schema.yaml`, `valid.yaml`, `invalid.yaml`) for the new scenarios; only add new fixtures if a scenario genuinely needs distinct content (e.g. a second invalid file with different error paths, to prove per-file attribution isn't just echoing the same file twice).

### 4. Docs

- `README.md` around line 175/182: update the printed `--help` transcript from `Usage: ys [OPTIONS] [FILE] [COMMAND]` / `[FILE]  The YAML file to validate` to reflect `[FILES]...` and the new doc comment text ("The YAML file(s) to validate"). Regenerate this block from actual `ys --help` output rather than hand-editing, to keep it accurate.
- Consider adding a short multi-file usage example near the existing single-file examples (~README.md lines 44/59/71/89) showing `ys -f schema.yaml file1.yaml file2.yaml`.

## Verification

1. `cargo build` — confirm the CLI compiles with `Vec<String>` positional.
2. `cargo test` — run the full suite, including the `cucumber`-based `features` target and `tests/ys_cli_json.rs`, confirming existing single-file scenarios still pass unchanged and new multi-file scenarios pass.
3. Manual smoke test from the repo root:
   ```
   cargo run --bin ys -- -f tests/fixtures/schema.yaml tests/fixtures/valid.yaml tests/fixtures/invalid.yaml
   cargo run --bin ys -- --json -f tests/fixtures/schema.yaml tests/fixtures/valid.yaml tests/fixtures/invalid.yaml
   cargo run --bin ys -- --fail-fast -f tests/fixtures/schema.yaml tests/fixtures/invalid.yaml tests/fixtures/valid.yaml
   ```
   Confirm: mixed valid/invalid shows exit code 1 with only the invalid file's errors (headered when human, `"file"`-tagged when JSON); `--fail-fast` stops after the first failing file.
