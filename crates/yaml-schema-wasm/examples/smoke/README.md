# yaml-schema-wasm smoke test

A small static page that loads the `yaml-schema-wasm` package built by `wasm-pack` and runs
`validate()`, `validateSchema()` and `version()` against fixed inputs. Each case's result is
checked, so a broken `pkg/` build shows up as a red **FAIL**.

## Build the wasm package

From the repository root:

```sh
wasm-pack build crates/yaml-schema-wasm --target web --out-dir pkg
```

This produces `crates/yaml-schema-wasm/pkg/`. It's gitignored because it's a generated build
artifact; rebuild it whenever `src/**` or `crates/yaml-schema-wasm/src/**` changes.

## Serve and open

Browsers block loading wasm modules from `file://` URLs, so this must be served over HTTP. From
the **repository root**:

```sh
python3 -m http.server 8000
```

Then open <http://localhost:8000/crates/yaml-schema-wasm/examples/smoke/index.html>.
