Feature: Object types

  Scenario: object
    Given a YAML schema:
      ```
      type: object
      ```
    Then it should accept:
      ```
      key: value
      another_key: another_value
      ```
    And it should accept:
      ```
      Sun: 1.9891e30
      Jupiter: 1.8986e27
      Saturn: 5.6846e26
      Neptune: 10.243e25
      Uranus: 8.6810e25
      Earth: 5.9736e24
      Venus: 4.8685e24
      Mars: 6.4185e23
      Mercury: 3.3022e23
      Moon: 7.349e22
      Pluto: 1.25e22
      ```
    # Unlike JSON, YAML allows numeric keys, which are treated as strings
    And it should accept:
      ```
      0.01: cm
      1: m
      1000: km
      ```
    But it should NOT accept:
      ```
      "Not an object"
      ```
    And it should NOT accept:
      ```
      ["An", "array", "not", "an", "object"]
      ```

  Scenario: properties
    Given a YAML schema:
      ```
      type: object
      properties:
        number:
          type: number
        street_name:
          type: string
        street_type:
          enum: [Street, Avenue, Boulevard]
      ```
    Then it should accept:
      ```
      number: 1600
      street_name: Pennsylvania
      street_type: Avenue
      ```
    # If we provide the number in the wrong type, it is invalid:
    But it should NOT accept:
      ```
      number: "1600"
      street_name: Pennsylvania
      street_type: Avenue
      ```
    # By default, leaving out properties is valid
    And it should accept:
      ```
      number: 1600
      street_name: Pennsylvania
      ```
    # By extension, even an empty object is valid
    And it should accept:
      ```
      {}
      ```
    # By default, providing additional properties is valid
    And it should accept:
      ```
      number: 1600
      street_name: Pennsylvania
      street_type: Avenue
      direction: NW
      ```

  Scenario: Pattern properties
    Given a YAML schema:
      ```
      type: object
      patternProperties:
        ^S_:
          type: string
        ^I_:
          type: integer
      ```
    Then it should accept:
      ```
      S_25: This is a string
      ```
    And it should accept:
      ```
      I_0: 42
      ```
    # If the name starts with S_, it must be a string
    But it should NOT accept:
      ```
      S_0: 42
      ```
    # If the name starts with I_, it must be an integer
    And it should NOT accept:
      ```
      I_42: This is a string
      ```
    # This is a key that doesn't match any of the regular expressions
    But it should accept:
      ```
      keyword: value
      ```

  Scenario: Additional properties
    Given a YAML schema:
      ```
      type: object
      properties:
        number:
          type: number
        street_name:
          type: string
        street_type:
          enum: [Street, Avenue, Boulevard]
      additionalProperties: false
      ```
    Then it should accept:
      ```
      number: 1600
      street_name: Pennsylvania
      street_type: Avenue
      ```
    # Since additionalProperties is false, an extra property "direction" makes the object invalid:
    But it should NOT accept:
      ```
      number: 1600
      street_name: Pennsylvania
      street_type: Avenue
      direction: NW
      ```

  Scenario: additionalProperties as a schema
    Given a YAML schema:
      ```
      type: object
      properties:
        number:
          type: number
        street_name:
          type: string
        street_type:
          enum: [Street, Avenue, Boulevard]
      additionalProperties:
        type: string
      ```
    Then it should accept:
      ```
      number: 1600
      street_name: Pennsylvania
      street_type: Avenue
      ```
    # This is valid, since the additional property's value is a string:
    And it should accept:
      ```
      number: 1600
      street_name: Pennsylvania
      street_type: Avenue
      direction: NW
      ```
    # This is invalid, since the additional property's value is not a string:
    And it should NOT accept:
      ```
      number: 1600
      street_name: Pennsylvania
      street_type: Avenue
      office_number: 201
      ```

  Scenario: Additional properties with properties and patternProperties
    Given a YAML schema:
      ```
      type: object
      properties:
        builtin:
          type: number
      patternProperties:
        ^S_:
          type: string
        ^I_:
          type: integer
      additionalProperties:
        type: string
      ```
    Then it should accept:
      ```
      builtin: 42
      ```
    # This is a key that doesn't match any of the regular expressions
    And it should accept:
      ```
      keyword: value
      ```
    # It must be a string, not an integer
    And it should NOT accept:
      ```
      keyword: 42
      ```

  Scenario: Required properties
    Given a YAML schema:
      ```
      type: object
      properties:
        name:
          type: string
        email:
          type: string
        address:
          type: string
        telephone:
          type: string
      required:
        - name
        - email
      ```
    Then it should accept:
      ```
      name: William Shakespeare
      email: bill@stratford-upon-avon.co.uk
      ```
    # Providing extra properties is fine, even properties not defined in the schema:
    And it should accept:
      ```
      name: William Shakespeare
      email: bill@stratford-upon-avon.co.uk
      address: Henley Street, Stratford-upon-Avon, Warwickshire, England
      authorship: in question
      ```
    # Missing the required "email" property makes the YAML document invalid:
    But it should NOT accept:
      ```
      name: William Shakespeare
      address: Henley Street, Stratford-upon-Avon, Warwickshire, England
      ```
    # A property with a null value is considered not being present:
    And it should NOT accept:
      ```
      name: William Shakespeare
      address: Henley Street, Stratford-upon-Avon, Warwickshire, England
      email: null
      ```

  Scenario: Property names
    Given a YAML schema:
      ```
      type: object
      propertyNames:
        pattern: "^[A-Za-z_][A-Za-z0-9_]*$"
      ```
    Then it should accept:
      ```
      _a_proper_token_001: "value"
      ```
    But it should NOT accept:
      ```
      -001 invalid: "value"
      ```
    And the error message should be "[1:1] .: Property name '-001 invalid' does not match pattern '^[A-Za-z_][A-Za-z0-9_]*$'"

  Scenario: Size
    Given a YAML schema:
      ```
      type: object
      minProperties: 2
      maxProperties: 3
      ```
    Then it should NOT accept:
      ```
      {}
      ```
    And it should NOT accept:
      ```
      a: 0
      ```
    But it should accept:
      ```
      a: 0
      b: 1
      ```
    And it should accept:
      ```
      a: 0
      b: 1
      c: 2
      ```
    But it should NOT accept:
      ```
      a: 0
      b: 1
      c: 2
      d: 3
      ```
