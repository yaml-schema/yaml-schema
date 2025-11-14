Feature: Multiple types

  @ignore
  Scenario: "type: [string, number]" should accept strings and numbers
    Given a YAML schema:
      ```
      type:
        - string
        - number
      ```
    Then it should accept:
      ```
      "I'm a string"
      ```
    And it should accept:
      ```
      42
      ```
    But it should NOT accept:
      ```
      an:
        - arbitrarily
        - nested
      data: structure
      ```
