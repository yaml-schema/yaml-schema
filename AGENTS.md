# Agent Guidelines for yaml-schema

YAML schema validator in Rust (edition 2024). This repository contains both:

- Library crate: `yaml-schema`
- CLI binary: `ys`

The project validates YAML files against YAML-defined schemas (similar in spirit to JSON Schema).

## File Organization

- `src/lib.rs` - library entry point
- `src/schemas/` - schema type definitions (one module per type)
- `src/validation/` - validation logic (kept separate from schema definitions)
- `src/loader.rs` - schema loading/parsing from YAML
- `src/engine.rs` - core validation engine
- `src/error.rs` - error types/macros (`thiserror`)
- `src/bin/ys.rs` - CLI binary
- `features/` - Cucumber BDD feature tests
- `tests/` - integration tests

## Rust Import Rules

- Group imports in this order: `std`, external crates, then `crate::`.
- Split multiple symbols from the same module into individual `use` statements.
  - Do not group multiple imported symbols in braces.

## Error Handling

Use `crate::Error` and `crate::Result<T>` consistently. Prefer existing error macros:

```rust
Err(generic_error!("msg: {}", val))
Err(expected_mapping!(marked_yaml))
Err(unsupported_type!("desc: {:?}", val))
```

Always include location markers via `format_marker()` from `utils`.

## Key Dependencies

- `saphyr` - YAML parsing with location-preserving `MarkedYaml<'a>`, plus `YamlData` and `Scalar`
- `hashlink` - `LinkedHashMap` for insertion-ordered mappings
- `ordered-float` - `OrderedFloat` for stable float comparison and ordering

## Schema Architecture

- `BaseSchema` - common fields shared by all schemas
- `TypedSchema` - enum for type-specific variants (Array, Object, String, Number, etc.)
- `Schema` - top-level enum; large variants use `Box` (for example `Schema::Object(Box<ObjectSchema>)`)
- Composition keywords supported: `allOf`, `anyOf`, `oneOf`, `not`
- Shared ownership uses `Rc` (single-threaded; see `RootSchema.schema`)
- Implement `std::fmt::Display` for schema types

## Core Traits and Patterns

### Loading

Implement `FromAnnotatedMapping<T>` and `TryFrom<&MarkedYaml>` for schema loading.

```rust
impl FromAnnotatedMapping<MySchema> for MySchema {
    fn from_annotated_mapping(mapping: &AnnotatedMapping<MarkedYaml>) -> Result<Self> { ... }
}
```

### Validation

Implement `Validator`; pass `&Context` for both error reporting and `$ref` resolution.

```rust
impl Validator for MySchema {
    fn validate(&self, context: &Context, value: &MarkedYaml) -> Result<()> { ... }
}
```

Validation should accumulate errors (rather than fail fast) unless `fail-fast` is enabled.

## Testing Expectations

- Unit tests: colocated in `#[cfg(test)]` modules inside `src/`
- Doctests: executable examples in public `///` docs
- BDD tests: Gherkin feature files in `features/` with step definitions in `tests/`

Run all tests with:

```bash
cargo test
```

## Pre-Commit Quality Checklist

1. `cargo fmt`
2. `cargo clippy` (fix warnings)
3. `cargo test`
4. Add or update Cucumber scenarios in `features/` for new schema behavior
5. Ensure error messages include location information
