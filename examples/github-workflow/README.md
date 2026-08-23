# GitHub Actions workflow example

`schema.yaml` is a hand conversion, into yaml-schema's native dialect, of the
canonical GitHub Actions workflow JSON Schema:
https://json.schemastore.org/github-workflow.json

Unlike `examples/kustomization/`, there's no `fixtures/` directory here —
the schema is validated directly against this repo's own workflow files in
[`.github/workflows/`](../../.github/workflows), so the example stays honest
and current as those workflows evolve.

Validate one workflow:

```sh
cargo run --bin ys -- -f examples/github-workflow/schema.yaml .github/workflows/ci-tests.yaml
```

Or validate all of them at once:

```sh
cargo run --bin ys -- -f examples/github-workflow/schema.yaml .github/workflows/*.yaml .github/workflows/*.yml
```
