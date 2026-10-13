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

  # An empty plain scalar (e.g. `key:` with no value) resolves to null under the YAML 1.2
  # core schema; it must not be confused with an explicit empty string.
  Scenario: Empty mapping value is null
    Given a YAML schema:
      ```
      type: object
      properties:
        a:
          type: "null"
      ```
    Then it should accept:
      ```
      a:
      ```
    And it should accept:
      ```
      a: ~
      ```
    And it should accept:
      ```
      a: null
      ```
    But it should NOT accept:
      ```
      a: ""
      ```
