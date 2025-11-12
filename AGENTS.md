# Agent Guidelines for yaml-schema

This document provides guidance for AI coding assistants working on the `yaml-schema` project.

## Project Overview

**yaml-schema** is a YAML schema validator written in Rust. It validates YAML files against schemas defined in YAML format, similar to JSON Schema but for YAML. The project is both a library (`yaml-schema`) and a CLI tool (`ys`).

### Key Components

- **Schemas** (`src/schemas/`): Schema type definitions (Array, Object, String, Number, etc.)
- **Validation** (`src/validation/`): Validation logic for different schema types
- **Loader** (`src/loader.rs`): Schema loading and parsing from YAML files
- **Engine** (`src/engine.rs`): Core validation engine
- **Error Handling** (`src/error.rs`): Custom error types using `thiserror`

## Rust Best Practices

### Code Style

1. **Edition**: This project uses Rust edition 2024
2. **Formatting**: Follow `rustfmt` defaults (run `cargo fmt` before committing)
3. **Naming Conventions**:
   - Use `snake_case` for functions, variables, and modules
   - Use `PascalCase` for types, traits, and enums
   - Use `SCREAMING_SNAKE_CASE` for constants
   - Use descriptive names that reflect the domain (schema validation)
4. **Imports**:
   - Group imports with imports from `std` in the first group, then imports from all other dependencies in the second group, then finally, imports from the same crate in the final group.
   - When importing multiple symbols from the same module, **SPLIT** them into individual `use` statements and _don't_ import them as a group.

### Error Handling

1. **Custom Error Type**: Use `crate::Error` (defined in `src/error.rs`) for all errors
2. **Error Macros**: The project uses custom macros for error creation:
   - `generic_error!()` - Generic errors with formatting
   - `expected_mapping!()` - Expected mapping errors
   - `unsupported_type!()` - Unsupported type errors
3. **Result Type**: Use `crate::Result<T>` which is an alias for `std::result::Result<T, Error>`
4. **Error Propagation**: Prefer `?` operator over explicit `match` when appropriate
5. **Error Context**: Always include location markers when available (use `format_marker()` from `utils`)

### Type System

1. **Enums**: Use enums for schema variants (`Schema`, `TypedSchema`, etc.)
2. **Traits**: Implement `TryFrom` for conversions that can fail
3. **Newtypes**: Use newtypes or enums to represent domain concepts (e.g., `Number`, `ConstValue`)
4. **Derive Macros**: Use `#[derive(Debug, PartialEq)]` for types that need comparison and debugging

### Ownership and Borrowing

1. **References**: Prefer borrowing (`&`) over owned values when possible
2. **Rc/Arc**: Use `Rc` for shared ownership within single-threaded contexts (see `RootSchema.schema`)
3. **Box**: Use `Box` for large types on the stack (e.g., `Schema::Object(Box<ObjectSchema>)`)
4. **Lifetimes**: Be explicit about lifetimes when working with `MarkedYaml<'a>` and `YamlData<'a>`

### Pattern Matching

1. **Match Expressions**: Use exhaustive `match` expressions for enums
2. **Guards**: Use match guards when additional conditions are needed
3. **if let**: Use `if let` for single-variant matches
4. **Destructuring**: Destructure structs and enums in match arms

### Testing

1. **Unit Tests**: Place unit tests in the same file using `#[cfg(test)]` modules
2. **Integration Tests**: Place integration tests in `tests/` directory
3. **Cucumber Tests**: The project uses Cucumber for BDD-style feature tests in `features/`
4. **Test Initialization**: Use `ctor` crate for test setup (see `lib.rs`)

### Dependencies

Key dependencies and their usage:

- **saphyr**: YAML parsing library (`MarkedYaml`, `YamlData`, `Scalar`)
- **thiserror**: Error handling (`#[derive(Error)]`)
- **eyre**: Error reporting (though `thiserror` is primary)
- **hashlink**: `LinkedHashMap` for preserving insertion order
- **ordered-float**: `OrderedFloat` for floating-point comparisons
- **serde**: Serialization (used for some data structures)
- **log**: Logging (use `debug!`, `info!`, etc.)

## Project-Specific Patterns

### Schema Loading

1. **FromAnnotatedMapping**: Implement `FromAnnotatedMapping<T>` trait for loading schemas from YAML mappings
2. **TryFrom**: Implement `TryFrom<&MarkedYaml>` for conversions
3. **Loader Module**: Use `loader::load_file()` and `loader::load_from_doc()` for loading schemas

### Validation

1. **Validator Trait**: Implement `Validator` trait for schema types
2. **Context**: Pass `&Context` to validation methods for error reporting and reference resolution
3. **Error Accumulation**: Collect validation errors rather than failing fast (unless `fail-fast` is enabled)

### Schema Types

1. **Base Schema**: All schemas extend `BaseSchema` which contains common fields
2. **Typed Schemas**: Use `TypedSchema` enum for type-specific schemas
3. **Composition**: Support `allOf`, `anyOf`, `oneOf`, and `not` for schema composition
4. **Display**: Implement `std::fmt::Display` for schema types for better error messages

### YAML Handling

1. **MarkedYaml**: Always use `MarkedYaml<'a>` for YAML values (preserves location information)
2. **YamlData**: Use `YamlData` enum to match YAML structure (Mapping, Sequence, Value)
3. **Scalar**: Use `Scalar` enum for scalar values (Null, Boolean, Integer, FloatingPoint, String)
4. **Location Preservation**: Always preserve location markers for better error messages

## Common Patterns

### Creating Schema Variants

```rust
// Use associated functions for constructors
impl Schema {
    pub fn object(schema: ObjectSchema) -> Schema {
        Schema::Object(Box::new(schema))
    }
}
```

### Error Creation

```rust
// Use project macros for errors
Err(generic_error!("Error message: {}", value))
Err(expected_mapping!(marked_yaml))
Err(unsupported_type!("Unsupported: {:?}", value))
```

### Validation Pattern

```rust
impl Validator for MySchema {
    fn validate(&self, context: &Context, value: &MarkedYaml) -> Result<()> {
        // Validation logic here
        // Return Ok(()) on success, Err(...) on failure
    }
}
```

### Loading Pattern

```rust
impl FromAnnotatedMapping<MySchema> for MySchema {
    fn from_annotated_mapping(mapping: &AnnotatedMapping<MarkedYaml>) -> Result<Self> {
        // Extract fields from mapping
        // Return constructed schema
    }
}
```

## Architecture Guidelines

1. **Separation of Concerns**: Keep schema definitions separate from validation logic
2. **Modularity**: Each schema type has its own module in `src/schemas/`
3. **Error Messages**: Always include location information in error messages when available
4. **Documentation**: Add doc comments for public APIs using `///`

## When Making Changes

1. **Run Tests**: Always run `cargo test` before committing
2. **Check Formatting**: Run `cargo fmt` to ensure consistent formatting
3. **Check Lints**: Run `cargo clippy` and fix warnings
4. **Update Features**: If adding new schema features, add Cucumber tests in `features/`
5. **Error Messages**: Ensure error messages are helpful and include location information
6. **Backward Compatibility**: Consider backward compatibility when changing APIs

## File Organization

- `src/lib.rs`: Library entry point, public API
- `src/schemas/`: Schema type definitions
- `src/validation/`: Validation logic
- `src/loader.rs`: Schema loading from YAML
- `src/engine.rs`: Validation engine
- `src/error.rs`: Error types and macros
- `src/bin/ys.rs`: CLI binary entry point
- `features/`: Cucumber feature files
- `tests/`: Integration tests

## Questions to Consider

When implementing new features:

1. Does it follow the existing schema pattern?
2. Are errors properly formatted with location information?
3. Is the validation logic testable?
4. Should it be added to Cucumber features?
5. Does it maintain backward compatibility?
6. Is the code idiomatic Rust?

## Additional Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [JSON Schema Specification](https://json-schema.org/) (for reference, as YAML Schema is based on JSON Schema)
- Project README: See `README.md` for user-facing documentation
