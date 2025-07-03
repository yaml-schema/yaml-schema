Feature: Not

  @const @not
  Scenario: not
    Given a YAML schema:
      ```
      not:
        type: number
        multipleOf: 2
      ```
    Then it should accept:
      ```
      1
      ```
    And it should accept:
      ```
      -1
      ```
    And it should accept:
      ```
      3
      ```
    But it should NOT accept:
      ```
      2
      ```
    And it should NOT accept:
      ```
      -2
      ```
