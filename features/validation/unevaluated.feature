Feature: Unevaluated properties and items (JSON Schema 2020-12)

  Scenario: unevaluatedProperties false with properties
    Given a YAML schema:
      ```
      type: object
      properties:
        a:
          type: string
      unevaluatedProperties: false
      ```
    Then it should accept:
      ```
      a: hello
      ```
    But it should NOT accept:
      ```
      a: hello
      b: extra
      ```

  Scenario: allOf merges evaluated names for unevaluatedProperties
    Given a YAML schema:
      ```
      allOf:
        - properties:
            a:
              type: string
        - unevaluatedProperties: false
      ```
    Then it should accept:
      ```
      a: x
      ```
    But it should NOT accept:
      ```
      a: x
      b: y
      ```

  Scenario: unevaluatedItems after prefixItems only
    Given a YAML schema:
      ```
      type: array
      prefixItems:
        - type: integer
      unevaluatedItems:
        type: string
      ```
    Then it should accept:
      ```
      - 1
      - foo
      - bar
      ```
    But it should NOT accept:
      ```
      - 1
      - 2
      ```

  Scenario: unevaluatedItems applies to all indices when no items or prefixItems
    Given a YAML schema:
      ```
      type: array
      unevaluatedItems:
        type: integer
      ```
    Then it should accept:
      ```
      - 1
      - 2
      ```
    But it should NOT accept:
      ```
      - 1
      - hi
      ```

  Scenario: anyOf successful branches merge annotations for unevaluatedProperties
    Given a YAML schema:
      ```
      anyOf:
        - properties:
            a:
              type: string
        - properties:
            b:
              type: string
      unevaluatedProperties: false
      ```
    Then it should accept:
      ```
      a: ok
      ```
    And it should accept:
      ```
      b: ok
      ```
    But it should NOT accept:
      ```
      a: ok
      c: extra
      ```
