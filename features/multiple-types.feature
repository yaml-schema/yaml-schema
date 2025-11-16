Feature: Multiple types

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
      null
      ```
    And it should NOT accept:
      ```
      true
      ```
    And it should NOT accept:
      ```
      an:
        - arbitrarily
        - nested
      data: structure
      ```

  Scenario: Multiple types with constraints
    Given a YAML schema:
      ```
      type:
        - string
        - number
      minimum: 1
      minLength: 1
      ```
    Then it should accept:
      ```
      1
      ```
    And it should accept:
      ```
      "one"
      ```
    But it should NOT accept:
      ```
      0
      ```
    And it should NOT accept:
      ```
      ""
      ```
