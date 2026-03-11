# Agent Guidelines for yaml-schema

YAML schema validator in Rust (edition 2024). Library (`yaml-schema`) + CLI (`ys`). Validates YAML files against YAML-defined schemas, similar to JSON Schema.

## File Organization

- `src/lib.rs` — library entry point
- `src/schemas/` — schema type definitions (one module per type)
- `src/validation/` — validation logic (separate from schema definitions)
- `src/loader.rs` — schema loading/parsing from YAML
- `src/engine.rs` — core validation engine
- `src/error.rs` — error types/macros (`thiserror`)
- `src/bin/ys.rs` — CLI binary
- `features/` — Cucumber BDD feature tests
- `tests/` — integration tests

## Import Rules

- Group: `std` first, external crates second, `crate::` last.
- **SPLIT** multiple symbols from the same module into individual `use` statements. Do NOT group them.

## Error Handling

Use `crate::Error` and `crate::Result<T>` everywhere. Macros:

```rust
Err(generic_error!("msg: {}", val))
Err(expected_mapping!(marked_yaml))
Err(unsupported_type!("desc: {:?}", val))
```

Always include location markers via `format_marker()` from `utils`.

## Key Dependencies

- **saphyr** — YAML parsing: `MarkedYaml<'a>` (location-preserving), `YamlData`, `Scalar` (Null/Boolean/Integer/FloatingPoint/String)
- **hashlink** — `LinkedHashMap` (preserves insertion order)
- **ordered-float** — `OrderedFloat` for float comparisons

## Schema Architecture

- `BaseSchema` — common fields shared by all schemas
- `TypedSchema` enum — type-specific variants (Array, Object, String, Number, etc.)
- `Schema` — top-level enum; large variants use `Box` (e.g., `Schema::Object(Box<ObjectSchema>)`)
- Composition: `allOf`, `anyOf`, `oneOf`, `not`
- Shared ownership via `Rc` (single-threaded; see `RootSchema.schema`)
- Implement `std::fmt::Display` for schema types

## Core Traits & Patterns

**Loading** — implement `FromAnnotatedMapping<T>` and `TryFrom<&MarkedYaml>`:

```rust
impl FromAnnotatedMapping<MySchema> for MySchema {
    fn from_annotated_mapping(mapping: &AnnotatedMapping<MarkedYaml>) -> Result<Self> { ... }
}
```

**Validation** — implement `Validator`; pass `&Context` for error reporting and `$ref` resolution:

```rust
impl Validator for MySchema {
    fn validate(&self, context: &Context, value: &MarkedYaml) -> Result<()> { ... }
}
```

Errors are accumulated (not fail-fast) unless `fail-fast` is enabled.

## Testing

- **Unit tests** — `#[cfg(test)]` modules inside `src/` source files, colocated with the code they test
- **Doctests** — executable examples in `///` doc comments on public functions and types
- **Cucumber BDD tests** — feature files in `features/` using Gherkin syntax; step definitions in `tests/`

Run all tests with `cargo test`. Cucumber tests run as part of the default test suite.

## Before Committing

1. `cargo fmt`
2. `cargo clippy` — fix warnings
3. `cargo test`
4. New schema features need Cucumber tests in `features/`
5. Error messages must include location information
