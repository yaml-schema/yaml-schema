# Deployment example

`schema.yaml` is a hand-authored schema modeling a practical subset of the
Kubernetes apps/v1 `Deployment` manifest — not a full conversion of the
upstream Kubernetes OpenAPI spec.

It reuses `ObjectMeta` and `PodSpec` from
[`examples/pod/schema.yaml`](../pod/schema.yaml) via a cross-file `$ref`
(e.g. `$ref: "../pod/schema.yaml#/$defs/PodSpec"`) instead of redefining
them, so a Deployment's pod template (`spec.template.spec`) is validated
against the exact same `PodSpec` definition as a standalone Pod.

`fixtures/` contains representative Deployment manifests: a minimal
deployment, one with container resource requests/limits and env vars, and
one using a `RollingUpdate` strategy with `configMap` volumes.

Validate a fixture:

```sh
cargo run --bin ys -- -f examples/deployment/schema.yaml examples/deployment/fixtures/simple-deployment.yaml
```

Or validate all of them at once:

```sh
cargo run --bin ys -- -f examples/deployment/schema.yaml examples/deployment/fixtures/*.yaml
```
