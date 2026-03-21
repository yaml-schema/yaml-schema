Feature: Booleans

  Scenario: Boolean type
    Given a YAML schema:
      ```
      type: boolean
      ```
    Then it should accept:
      ```
      true
      ```
    And it should accept:
      ```
      false
      ```
    But it should NOT accept:
      ```
      "true"
      ```
