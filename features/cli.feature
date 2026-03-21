Feature: CLI usage

  Scenario: Display the version
    When the following command is run:
      ```
      ys version
      ```
    Then it should exit with status code 0
    And it should output:
      ```
      ys 0.9.1
      ```

  Scenario: Basic validation with a valid file
    When the following command is run:
      ```
      ys -f tests/fixtures/schema.yaml tests/fixtures/valid.yaml
      ```
    Then it should exit with status code 0

  Scenario: Validation using top-level $schema instead of -f
    When the following command is run:
      ```
      ys tests/fixtures/instance_with_dollar_schema_valid.yaml
      ```
    Then it should exit with status code 0

  Scenario: Validation using $schema with an invalid instance
    When the following command is run:
      ```
      ys tests/fixtures/instance_with_dollar_schema_invalid.yaml
      ```
    Then it should exit with status code 1
    And stderr output should end with:
      ```
      [2:6] .foo: Expected a string, but got: 42 (int)
      [3:6] .bar: Expected a number, but got: "I'm a string" (string)
      ```

  Scenario: Basic validation with an invalid file
    When the following command is run:
      ```
      ys -f tests/fixtures/schema.yaml tests/fixtures/invalid.yaml
      ```
    Then it should exit with status code 1
    And stderr output should end with:
      ```
      [1:6] .foo: Expected a string, but got: 42 (int)
      [2:6] .bar: Expected a number, but got: "I'm a string" (string)
      ```

  Scenario: Basic validation with an invalid file and JSON output
    When the following command is run:
      ```
      ys --json -f tests/fixtures/schema.yaml tests/fixtures/invalid.yaml
      ```
    Then it should exit with status code 1
    And stdout should be a JSON array with two validation errors for paths foo and bar
