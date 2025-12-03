Feature: Basic YAML schema

  Scenario: Empty YAML
    Given a YAML schema:
      ```
      
      ```
    Then it should accept:
      ```
      42
      ```
    And it should accept:
      ```
      "I'm a string"
      ```
    And it should accept:
      ```
      an:
        - arbitrarily
        - nested
      data: structure
      ```

  Scenario: `true` should accept anything
    Given a YAML schema:
      ```
      true
      ```
    Then it should accept:
      ```
      42
      ```
    And it should accept:
      ```
      "I'm a string"
      ```
    And it should accept:
      ```
      an:
        - arbitrarily
        - nested
      data: structure
      ```

  Scenario: `false` should reject anything
    Given a YAML schema:
      ```
      false
      ```
    Then it should NOT accept:
      ```
      42
      ```
    And it should NOT accept:
      ```
      "I'm a string"
      ```
    And it should NOT accept:
      ```
      an:
        - arbitrarily
        - nested
      data: structure
      ```

  Scenario: "type: foo" should error
    Given a YAML schema:
      ```
      type: foo
      ```
    Then it should fail with "Unsupported type: Expected type: string, number, integer, object, or array, but got: foo"

  Scenario: "type: string" should accept strings
    Given a YAML schema:
      ```
      type: string
      ```
    Then it should accept:
      ```
      ""
      ```
    And it should accept:
      ```
      "I'm a string"
      ```
    But it should NOT accept:
      ```
      42
      ```
    And it should NOT accept:
      ```
      an:
        - arbitrarily
        - nested
      data: structure
      ```

  Scenario: "type: number" should accept numbers
    Given a YAML schema:
      ```
      type: number
      ```
    Then it should accept:
      ```
      42
      ```
    And it should accept:
      ```
      3.14
      ```
    But it should NOT accept:
      ```
      "I'm a string"
      ```
    And it should NOT accept:
      ```
      an:
        - arbitrarily
        - nested
      data: structure
      ```

  Scenario: "type: object" should validate properties
    Given a YAML schema:
      ```
      type: object
      properties:
        foo:
          type: string
        bar:
          type: number
      ```
    Then it should accept:
      ```
      foo: "I'm a string"
      bar: 42
      ```
    # missing properties should be allowed, unless required properties are specified
    And it should accept:
      ```
      foo: "I'm a string"
      ```
    But it should NOT accept:
      ```
      foo: 42
      bar: "I'm a string"
      ```
    And the error message should be "[1:6] .foo: Expected a string, but got: Value(Integer(42))"
