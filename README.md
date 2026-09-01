ys - yaml-schema
====

[![CI Tests](https://github.com/yaml-schema/yaml-schema/actions/workflows/ci-tests.yaml/badge.svg)](https://github.com/yaml-schema/yaml-schema/actions/workflows/ci-tests.yaml)
[![Crates.io](https://img.shields.io/crates/v/yaml-schema.svg)](https://crates.io/crates/yaml-schema)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**yaml-schema** is a tool to validate YAML files against a YAML schema.

The YAML schema specification is based on [JSON Schema](https://json-schema.org/). The difference is that both the schema and the data it describes are plain YAML, so you get comments, anchors, and multi-line strings for free, with no lossy conversion to/from JSON.

**yaml-schema** is both a Rust library and an executable (`ys`) which tells you exactly what's wrong: which value, on which line and column, and why. You can drop the CLI straight into CI or a pre-commit hook, or embed the Rust library directly.

See detailed documentation at [https://yaml-schema.net/](https://yaml-schema.net/).

## Why yaml-schema?

- **Full JSON Schema semantics** — write schemas using the keywords you already know (`$ref`, `type`, `properties`, `oneOf`/`anyOf`/`allOf`, etc.), just expressed in YAML instead of JSON.
- **Author schemas *and* data in YAML** — no round-tripping through JSON, so comments, anchors, and multi-line strings stay intact.
- **Precise, actionable errors** — every failure reports a byte offset, line, column, and dot-separated path to the offending value, so you can pinpoint the problem immediately.
- **CI and agent friendly `--json` output** — structured, machine-readable errors on stdout, distinct from tooling failures on stderr, ready to wire into pipelines or agent workflows.
- **Self-validating** — the project validates its own configuration schema against itself as part of its test suite, so the tool is a live example of itself in action.

## Example Usage

Given a `schema.yaml` file containing:

```yaml
type: object
properties:
  foo:
    type: string
  bar:
    type: number
```

And a `valid.yaml` file containing:

```yaml
foo: "I'm a string"
bar: 42
```

Then when you issue the command

```
ys -f schema.yaml valid.yaml
```

Then the command should succeed with exit code 0

On the other hand, when given an `invalid.yaml` file containing:

```yaml
foo: 42
bar: "I'm a string"
```

Then the command

```
ys -f schema.yaml invalid.yaml
```

Should fail with exit code 1

## Installation

Currently, **yaml-schema** requires Git, Rust and Cargo to build and
install: [https://doc.rust-lang.org/cargo/](https://doc.rust-lang.org/cargo/)

To install the stable release from [crates.io](https://crates.io/crates/yaml-schema):

```
cargo install yaml-schema
```

That should build and install the executable at `$HOME/.cargo/bin/ys` (which should be in your PATH)

Alternatively, one can install from latest source:

```
cargo install --git https://github.com/yaml-schema/yaml-schema
```

## Usage

Running `ys` without any options or arguments should display the help:

```
A tool for validating YAML against a schema

Usage: ys [OPTIONS] [FILE] [COMMAND]

Commands:
  version  Display the ys version
  help     Print this message or the help of the given subcommand(s)

Arguments:
  [FILE]  The YAML file to validate

Options:
  -f, --schema <SCHEMAS>  Schema file(s) to load. The first is the root schema; additional
                          schemas are pre-loaded for $ref resolution. May be specified multiple
                          times (-f a.yaml -f b.yaml). Omit when the instance YAML has a
                          top-level string `$schema` (URL or path)
      --fail-fast         Specify this flag to exit (1) as soon as any error is encountered
      --json              Emit errors as JSON: validation failures as a JSON array on stdout;
                          other failures as {"error":"..."} on stderr
  -h, --help              Print help
  -V, --version           Print version
  ```

## JSON Output

Pass `--json` to emit structured errors instead of plain text. Use it with the same options as usual.

**Successful validation** (exit code `0`): stdout is empty.

```
ys --json -f schema.yaml valid.yaml
```

(no output on stdout)

**Validation failures** (exit code `1`): stdout is a single JSON **array** of objects, one per error. Each object has:

| Field   | Meaning |
|--------|---------|
| `index` | Byte offset into the source, or `null` if unknown |
| `line`  | 1-based line number, or `null` if unknown |
| `col`   | 0-based column index from the parser, or `null` if unknown |
| `path`  | Dot-separated path from the document root (e.g. `foo`, `items.0`) |
| `error` | Human-readable message |

Using the same `schema.yaml` / `invalid.yaml` scenario as [above](#example-usage), with `foo` and `bar` violating their types:

```sh
ys --json -f schema.yaml invalid.yaml
```

stdout (pretty-printed; the tool emits compact JSON on one line):

```json
[
  {
    "col": 5,
    "error": "Expected a string, but got: 42 (int)",
    "index": 5,
    "line": 1,
    "path": "foo"
  },
  {
    "col": 5,
    "error": "Expected a number, but got: \"I'm a string\" (string)",
    "index": 13,
    "line": 2,
    "path": "bar"
  }
]
```

**Other failures** (exit code `1`): schema load errors, missing arguments, YAML parse errors, and similar issues print a single JSON object on **stderr**: `{"error":"<message>"}`.

If the schema file cannot be read:

```sh
ys --json -f /path/to/missing-schema.yaml valid.yaml
```

stderr:

```json
{"error":"Failed to read YAML schema file /path/to/missing-schema.yaml: No such file or directory (os error 2)"}
```

The exact `error` text depends on the failure (OS messages, parse errors, etc.).

Validation errors are written to **stdout**; non-validation errors use **stderr**, so callers can distinguish validation results from tooling or I/O failures.

## YAML-Specific Notes

**JSON Schema vs YAML:** JSON object keys are always strings. YAML allows other **scalar** mapping keys (e.g. integers, booleans).

When writing schemas or instances in YAML, remember that **mapping keys are parsed by YAML first**. Unquoted keys such as `1` become a number. Keys that start with `@`, `#`, or other special characters may be invalid or require [quoting](https://stackoverflow.com/questions/19109912/do-i-need-quotes-for-strings-in-yaml). Use explicit quotes (e.g. `"@id"`, `"1"`) when the property name must be that exact string (see also [issue #62](https://github.com/yaml-schema/yaml-schema/issues/62)).

The `propertyNames` keyword validates each mapping key against a subschema. Only scalar types are permitted (`string`, `integer`, `number`, `boolean`, `null`); `array` and `object` types (and array/object keywords such as `items` or `properties`) are rejected at load time.

Composition keywords (`oneOf`, `anyOf`, `allOf`) are supported when every branch uses scalar types. When no `type` is provided, the subschema is treated as `type: string` and validates the canonical string form of the key (JSON Schema compatible).

String keywords such as `pattern` and `enum` work without an explicit `type`. When a non-string scalar `type` is specified (e.g. `integer`), the YAML key node is validated directly.

See the [Types](https://yaml-schema.net/features/types.html) documentation for details.

## Test Suite

**yaml-schema** uses [Cucumber](https://cucumber-rs.github.io/cucumber/main/) to specify and test its behavior:

- [CLI usage](features/cli.feature)
- [Basic features](features/basics.feature)
- [String validation](features/strings.feature)
- [Numeric types](features/numeric.feature)
- [Const](features/const.feature)
- [Enums](features/enums.feature)
- [Object types](features/objects.feature) (includes `propertyNames`)
- [Arrays](features/arrays.feature)
- [Composition](features/composition.feature)
- [Unevaluated properties/items](features/unevaluated.feature)

See the [features](features/) folder for all examples.

## Self-Validation

**yaml-schema** is _self-validating_. That is, running

```
cargo run -- -f yaml-schema.yaml yaml-schema.yaml
```

_should_ always succeed.

## License

MIT — see [LICENSE](LICENSE) for details.
