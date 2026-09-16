# Pod example

`schema.yaml` is a hand-authored schema modeling a practical subset of the
Kubernetes core/v1 `Pod` manifest (`apiVersion`, `kind`, `metadata`, and a
`spec` with `containers`, `initContainers`, `volumes`, etc.) — not a full
conversion of the upstream Kubernetes OpenAPI spec. It exists both as a
standalone example and as the source of `PodSpec`/`ObjectMeta` reused by
`examples/deployment/schema.yaml` via a cross-file `$ref`.

`fixtures/` contains representative Pod manifests: a minimal single-container
pod, a multi-container pod with an init container and resource limits, and a
pod using `configMap`/`emptyDir` volumes.

Validate a fixture:

```sh
cargo run --bin ys -- -f examples/pod/schema.yaml examples/pod/fixtures/simple-pod.yaml
```

Or validate all of them at once:

```sh
cargo run --bin ys -- -f examples/pod/schema.yaml examples/pod/fixtures/*.yaml
```
