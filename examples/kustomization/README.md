# kustomization.yaml example

`schema.yaml` is a hand conversion, into yaml-schema's native dialect, of the
SchemaStore JSON Schema for Kubernetes `kustomize`'s `kustomization.yaml`:
https://json.schemastore.org/kustomization.json

`fixtures/` contains real `kustomization.yaml` files pulled from the
[kubernetes-sigs/kustomize](https://github.com/kubernetes-sigs/kustomize)
`examples/` tree, used to verify the schema against real-world input.

Validate a fixture:

```sh
cargo run --bin ys -- -f examples/kustomization/schema.yaml examples/kustomization/fixtures/kustomization-simple.yaml
```

Or validate all of them at once:

```sh
cargo run --bin ys -- -f examples/kustomization/schema.yaml examples/kustomization/fixtures/*.yaml
```
