Feature: Numeric types

  Scenario: integer
    Given a YAML schema:
      ```
      type: integer
      ```
    Then it should accept:
      ```
      42
      ```
    And it should accept:
      ```
      -1
      ```
    And it should accept:
      ```
      1.0
      ```
    But it should NOT accept:
      ```
      3.1415926
      ```
    And it should NOT accept:
      ```
      "42"
      ```

  Scenario: Multiples
    Given a YAML schema:
      ```
      type: number
      multipleOf: 10
      ```
    Then it should accept:
      ```
      0
      ```
    And it should accept:
      ```
      10
      ```
    And it should accept:
      ```
      20
      ```
    But it should NOT accept:
      ```
      23
      ```

  Scenario: Range
    Given a YAML schema:
      ```
      type: number
      minimum: 0
      exclusiveMaximum: 10
      ```
    # Less than `minimum`
    Then it should not accept:
      ```
      -1
      ```
    # `minimum` is inclusive, so 0 is valid
    But it should accept:
      ```
      0
      ```
    And it should accept:
      ```
      10
      ```
    And it should accept:
      ```
      99
      ```
    # `exclusiveMaximum` is exclusive, so 100 is not valid
    But it should not accept:
      ```
      100
      ```
    # Greater than `maximum`
    And it should not accept:
      ```
      101
      ```