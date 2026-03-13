Feature: References ($ref)

  Scenario: Simple $ref to $defs
    Given a YAML schema:
      ```
      $defs:
        name:
          type: string
      type: object
      properties:
        name:
          $ref: "#/$defs/name"
      ```
    Then it should accept:
      ```
      name: "Alice"
      ```
    But it should NOT accept:
      ```
      name: 42
      ```

  Scenario: Direct circular $ref
    Given a YAML schema:
      ```
      $defs:
        a:
          $ref: "#/$defs/a"
      $ref: "#/$defs/a"
      ```
    Then it should NOT accept:
      ```
      anything
      ```
    And the error message should be "[1:1] .: Circular $ref detected: #/$defs/a"

  Scenario: Indirect circular $ref
    Given a YAML schema:
      ```
      $defs:
        a:
          $ref: "#/$defs/b"
        b:
          $ref: "#/$defs/a"
      $ref: "#/$defs/a"
      ```
    Then it should NOT accept:
      ```
      anything
      ```
    And the error message should be "[1:1] .: Circular $ref detected: #/$defs/a"

  Scenario: $ref to a remote full URI with fragment
    Given a YAML schema:
      ```
      type: object
      properties:
        kind:
          $ref: "https://yaml-schema.net/yaml-schema.yaml#/$defs/valid_types"
      ```
    Then it should accept:
      ```
      kind: string
      ```
    And it should accept:
      ```
      kind: object
      ```
    But it should NOT accept:
      ```
      kind: foobar
      ```
    And it should NOT accept:
      ```
      kind: 42
      ```
