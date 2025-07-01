Feature: CLI usage

  Scenario: Display the version
    When the following command is run:
      ```
      ys version
      ```
    Then it should exit with status code 0
    And it should output:
      ```
      ys 0.6.0
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
