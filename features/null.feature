Feature: Nulls

  Scenario: Null type
    Given a YAML schema:
      ```
      type: null
      ```
    Then it should accept:
      ```
      null
      ```
    But it should NOT accept:
      ```
      false
      ```
    And it should NOT accept:
      ```
      0
      ```
    And it should NOT accept:
      ```
      ""
      ```
