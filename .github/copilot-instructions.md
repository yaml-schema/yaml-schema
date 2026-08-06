<!-- okf-rs:begin -->
## Knowledge base

This project's structural knowledge — modules, types, functions, and their call graph — is available as an [OKF](https://cloud.google.com/blog/products/data-analytics/how-the-open-knowledge-format-can-improve-data-sharing) bundle in `knowledge/`. It's plain markdown with YAML frontmatter; browse `knowledge/index.md` for an overview, or query it with the CLI:

- `okf-rs search <query>` — find a symbol, type, or module by name or tag
- `okf-rs graph callers <id>` / `okf-rs graph callees <id>` — trace the call graph from a concept id (ids are shown by `search`)
- `okf-rs graph api` — list the public API surface
- `okf-rs graph cycles` — list call-graph cycles

Regenerate the bundle after code changes with `okf-rs generate`.
<!-- okf-rs:end -->
