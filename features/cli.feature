Feature: CLI usage

  Scenario: Display the version
    When the following command is run:
      ```
      ys version
      ```
    Then it should exit with status code 0
    And it should output:
      ```
      ys 0.8.0
      ```

  Scenario: Basic validation with a valid file
    When the following command is run:
      ```
      ys -f tests/fixtures/schema.yaml tests/fixtures/valid.yaml
      ```
    Then it should exit with status code 0

  Scenario: Basic validation with an invalid file
    When the following command is run:
      ```
      ys -f tests/fixtures/schema.yaml tests/fixtures/invalid.yaml
      ```
    Then it should exit with status code 1
    And stderr output should end with:
      ```
      [1:6] .foo: Expected a string, but got: Value(Integer(42))
      [2:6] .bar: Expected a number, but got: Value(String("I'm a string"))
      ```
