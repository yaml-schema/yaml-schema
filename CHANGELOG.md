# Changelog

All notable changes to this project are documented here. Release notes for published versions also appear on [GitHub Releases](https://github.com/yaml-schema/yaml-schema/releases).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.9.1] - 2026-03-21

### Added

- Array constraints: `minItems`, `maxItems`, and `uniqueItems` ([#44](https://github.com/yaml-schema/yaml-schema/pull/44)); `minContains` and `maxContains` ([#45](https://github.com/yaml-schema/yaml-schema/pull/45)).
- External `$ref` resolution via URIs, base URI handling for relative references, and related loader API updates ([#46](https://github.com/yaml-schema/yaml-schema/pull/46)).
- String `format` validation (date/time, network, URI, uuid, json-pointer, and others) ([#47](https://github.com/yaml-schema/yaml-schema/pull/47)).
- Conditional validation (`if` / `then` / `else`) ([#48](https://github.com/yaml-schema/yaml-schema/pull/48)); `dependentRequired` and `dependentSchemas`; inferred `object` / `string` types when keywords imply them ([#50](https://github.com/yaml-schema/yaml-schema/pull/50)).
- `unevaluatedProperties` and `unevaluatedItems` with annotation tracking across composition ([#53](https://github.com/yaml-schema/yaml-schema/pull/53)).
- CLI: `--json` for structured validation errors; clearer type-mismatch messages ([#54](https://github.com/yaml-schema/yaml-schema/pull/54)); validate instances using top-level `$schema` when `-f` is omitted ([#55](https://github.com/yaml-schema/yaml-schema/pull/55)).

### Changed

- Object validation: both `properties` and `patternProperties` apply to the same instance ([#51](https://github.com/yaml-schema/yaml-schema/pull/51)).

### Fixed

- `minLength` / `maxLength` use Unicode scalar counts ([#52](https://github.com/yaml-schema/yaml-schema/pull/52)).

## [0.9.0] - 2026-03-11

Major update: schema architecture refactor, new validation features, bug fixes, and expanded test coverage. Approximately **47 files changed**, ~3,900 insertions and ~3,400 deletions since v0.8.0.

### Added

- Circular `$ref` detection in schema validation ([#40](https://github.com/yaml-schema/yaml-schema/pull/40)) to prevent infinite loops during schema validation.
- `NumericBounds` shared trait for integer/number `min` / `max` / `exclusiveMin` / `exclusiveMax`.
- `patternProperties` support with correct validation precedence ([#37](https://github.com/yaml-schema/yaml-schema/pull/37), [#39](https://github.com/yaml-schema/yaml-schema/pull/39)).
- Extended `const` validation with a new `ConstValue` type, including null support ([#41](https://github.com/yaml-schema/yaml-schema/pull/41)).
- Full `oneOf` composition validation with dedicated feature tests.
- `numeric.rs` for shared bounds logic between integer and number schemas.
- New Cucumber feature files: `const.feature`, `objects.feature`, `one_of.feature`, `references.feature`.
- CLI integration tests using `assert_cmd` with step definitions for command execution, exit codes, and stdout/stderr assertions.
- `ys_vs_boon` benchmark suite.
- `.clippy.toml` configuration.
- Dependencies: `assert_cmd`, `jsonptr`, `ordered-float`.

### Changed

- **Schema architecture** ([#33](https://github.com/yaml-schema/yaml-schema/pull/33)): consolidated schema types; removed `base.rs`, `bool_or_typed.rs`, `const.rs`, and `typed_schema.rs`.
- Heavily reworked `yaml_schema.rs` as the central schema definition.
- Simplified `schemas.rs` module organization.
- Rewrote `loader.rs` for cleaner schema loading and parsing.
- Updated `bytes` from 1.10.1 to 1.11.1 ([#30](https://github.com/yaml-schema/yaml-schema/pull/30)).
- Expanded feature tests for numbers and composition.
- README: removed “Built by Windsurf” badge.

### Fixed

- Integer `min` / `max` comparison operators ([#32](https://github.com/yaml-schema/yaml-schema/pull/32)).
- Object validation precedence for pattern properties ([#37](https://github.com/yaml-schema/yaml-schema/pull/37)).
- Ignore `$schema` on instance data during object validation ([#34](https://github.com/yaml-schema/yaml-schema/pull/34)).
- Typos, engine context simplification, and related cleanup ([#41](https://github.com/yaml-schema/yaml-schema/pull/41)).

### Release

- **v0.9.0-rc1**: NumericBounds, PatternProperty, and extended const validation ([#39](https://github.com/yaml-schema/yaml-schema/pull/39)).
- **v0.9.0**: CLI Cucumber step definitions.

[0.9.1]: https://github.com/yaml-schema/yaml-schema/releases/tag/v0.9.1
[0.9.0]: https://github.com/yaml-schema/yaml-schema/releases/tag/v0.9.0
