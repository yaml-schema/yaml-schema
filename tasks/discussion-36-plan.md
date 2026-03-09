# Task Plan: Clarify Nested additionalProperties Behavior

## Source
- Type: GitHub Discussion
- Input: https://github.com/yaml-schema/yaml-schema/discussions/36

## Understanding
- A user expects `additionalProperties: false` at the root object schema to also reject unknown keys inside nested objects under `cluster.control[].*`.
- Current behavior accepts `foo: bar` inside array item objects because `additionalProperties` is scoped per object schema and is not inherited into nested object schemas unless explicitly declared there.
- Acceptance signal: the project should make this behavior clear and/or provide guidance so users do not interpret it as inconsistent validation.
- Edge cases to account for:
  - Nested objects inside `properties`
  - Objects under `items` in arrays
  - Interaction with `patternProperties` and `additionalProperties` precedence
- Unknown to confirm during implementation: whether maintainers want a docs-only clarification or a schema-language enhancement for inherited defaults (the latter would be a behavior change).

## Relevant Codebase Context
- `src/schemas/object.rs`
  - Parses `additionalProperties` into `ObjectSchema.additional_properties` as `BooleanOrSchema`.
  - Confirms the property is local to each parsed object schema instance.
- `src/validation/objects.rs`
  - Enforces `additionalProperties` only when validating the current `ObjectSchema` mapping.
  - Validates nested values via child schema validation, so nested objects need their own `additionalProperties`.
- `features/validation/objects.feature`
  - Already contains scenarios for:
    - `additionalProperties: false`
    - `additionalProperties` as a schema
    - interaction with `patternProperties`
  - Does not currently include an explicit nested-array-object scoping scenario mirroring Discussion #36.
- `yaml-schema.yaml`
  - Meta-schema currently models `additionalProperties` with `oneOf: [boolean, $ref: #/$defs/array_of_schemas]`; this may need review if docs or behavior are adjusted.

## Related Past PRs (up to 5)
- [#37 Fix patternProperties precedence over additionalProperties](https://github.com/yaml-schema/yaml-schema/pull/37): Recent object-validation fix in the same `additionalProperties` decision path.
- [#34 Ignore `$schema` property in data during object validation](https://github.com/yaml-schema/yaml-schema/pull/34): Adds object-validation rule exceptions and test coverage in `features/validation/objects.feature`.
- [#33 Refactor schema architecture: consolidate types, improve error handling, add benchmarks](https://github.com/yaml-schema/yaml-schema/pull/33): Large refactor that reshaped schema/validation architecture used by current object handling.
- [#21 v0.8](https://github.com/yaml-schema/yaml-schema/pull/21): Earlier release PR that includes object/type behavior changes and can provide historical context.
- [#20 fix: accept description at object](https://github.com/yaml-schema/yaml-schema/pull/20): Object-schema-focused change touching parsing behavior and expectations around object fields.

## Implementation Plan
1. Reproduce Discussion #36 exactly as a regression fixture in `features/validation/objects.feature` with two explicit outcomes:
   - root-only `additionalProperties: false` still accepts nested unknown key (`foo`) in array item object.
   - adding `additionalProperties: false` to the nested item object rejects `foo`.
2. Add/extend user-facing docs (README and/or dedicated docs file if present) with a short "scoping rules" note:
   - `additionalProperties` applies only to the object schema where it is declared.
   - show nested-object example for `properties` and `items`.
3. Validate meta-schema/docs alignment for `additionalProperties` typing in `yaml-schema.yaml`; if necessary, open a follow-up task if intended behavior and meta-schema diverge.
4. Optionally add a CLI-facing help/example note (if docs placement is maintainers' preferred style) to reduce repeated confusion from new users.

## Validation Plan
- Tests to add/update:
  - Update `features/validation/objects.feature` with a nested-array-object `additionalProperties` scoping scenario.
  - If behavior changes are made (not expected for this task), add/adjust unit tests in `src/validation/objects.rs`.
- Manual verification steps:
  - Run `cargo test` and ensure the new scenario passes.
  - Validate the two schema variants from Discussion #36 using `ys -f schema.yaml sample.yaml` and verify expected pass/fail outcomes.

## Risks / Open Questions
- Scope risk: introducing inherited `additionalProperties` would be a breaking semantic change and should not be done implicitly in a clarification task.
- Documentation placement is ambiguous (README vs dedicated docs page); confirm maintainer preference.
- The meta-schema definition for `additionalProperties` may not fully match implementation expectations; if mismatched, treat as a separate, explicitly scoped change.
