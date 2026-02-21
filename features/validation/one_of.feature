Feature: oneOf

  Scenario: oneOf with objects with types
    Given a YAML schema:
      ```
      oneOf:
        - type: object
          properties:
            type:
              const: "integer"
            minimum:
              type: integer
            maximum:
              type: integer
            multipleOf:
              type: integer
            exclusiveMinimum:
              type: integer
            exclusiveMaximum:
              type: integer
          required:
            - type
        - type: object
          properties:
            type:
              const: "string"
          required:
            - type
      ```
    Then it should accept:
      ```
      type: integer
      ```
    And it should accept:
      ```
      type: integer
      minimum: 1
      maximum: 10
      multipleOf: 2
      exclusiveMinimum: 0
      exclusiveMaximum: 11
      ```
    And it should accept:
      ```
      type: string
      ```
    But it should NOT accept:
      ```
      type: boolean
      ```
