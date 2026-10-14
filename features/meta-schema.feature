Feature: Meta-schema validation

  Scenario: The meta-schema validates itself
    When the following command is run:
      ```
      ys -f yaml-schema.yaml yaml-schema.yaml
      ```
    Then it should exit with status code 0

  Scenario: Nested schemas may use any supported keyword
    When the following command is run:
      ```
      ys -f yaml-schema.yaml tests/fixtures/meta_schema_valid_nested.yaml
      ```
    Then it should exit with status code 0

  Scenario: A misspelled keyword in a nested schema is rejected
    When the following command is run:
      ```
      ys -f yaml-schema.yaml tests/fixtures/meta_schema_nested_typo.yaml
      ```
    Then it should exit with status code 1
    And stderr output should end with:
      ```
      [4:5] .properties.number: Additional property 'typxe' is not allowed!
      ```
