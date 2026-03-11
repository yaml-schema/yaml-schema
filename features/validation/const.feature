Feature: Constant values

  @const
  Scenario: const
    Given a YAML schema:
      ```
      type: object
      properties:
        country:
          const: United States of America
      ```
    Then it should accept:
      ```
      country: United States of America
      ```
    But it should NOT accept:
      ```
      country: Canada
      ```

  Scenario: const with array value
    Given a YAML schema:
      ```
      type: object
      properties:
        coordinates:
          const: [1, 2]
      ```
    Then it should accept:
      ```
      coordinates: [1, 2]
      ```
    But it should NOT accept:
      ```
      coordinates: [1, 3]
      ```
    And it should NOT accept:
      ```
      coordinates: [1, 2, 3]
      ```

  Scenario: const with object value
    Given a YAML schema:
      ```
      type: object
      properties:
        config:
          const:
            env: production
            debug: false
      ```
    Then it should accept:
      ```
      config:
        env: production
        debug: false
      ```
    But it should NOT accept:
      ```
      config:
        env: development
        debug: false
      ```
    And it should NOT accept:
      ```
      config:
        env: production
      ```
