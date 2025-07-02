@arrays
Feature: Arrays

  Scenario: Array type
    Given a YAML schema:
      ```
      type: array
      ```
    Then it should accept:
      ```
      - 1
      - 2
      - 3
      - 4
      - 5
      ```
    And it should accept:
      ```
      - 3
      - different
      - types: "of values"
      ```
    But it should NOT accept:
      ```
      Not: "an array"
      ```

  Scenario: Array items
    Given a YAML schema:
      ```
      type: array
      items:
        type: number
      ```
    Then it should accept:
      ```
      - 1
      - 2
      - 3
      - 4
      - 5
      ```
    # A single non-number causes the entire array to be invalid
    But it should NOT accept:
      ```
      - 1
      - 2
      - "3"
      - 4
      - 5
      ```
    # The empty array is always valid
    And it should accept:
      ```
      []
      ```

  Scenario: Tuple validation
    Given a YAML schema:
      ```
      type: array
      prefixItems:
        - type: number
        - type: string
        - enum:
          - Street
          - Avenue
          - Boulevard
        - enum:
          - NW
          - NE
          - SW
          - SE
      ```
    Then it should accept:
      ```
      - 1600
      - Pennsylvania
      - Avenue
      - NW
      ```
    # "Drive" is not one of the acceptable street types
    But it should NOT accept:
      ```
      - 24
      - Sussex
      - Drive
      ```
    # This address is missing a street number
    And it should NOT accept:
      ```
      - Palais de l'Élysée
      ```
    # It's ok to not provide all of the items
    But it should accept:
      ```
      - 10
      - Downing
      - Street
      ```
    # And, by default, it's also ok to provide additional items
    And it should accept:
      ```
      - 1600
      - Pennsylvania
      - Avenue
      - NW
      - Washington
      ```

  Scenario: Additional Items
    Given a YAML schema:
      ```
      type: array
      prefixItems:
        - type: number
        - type: string
        - enum:
          - Street
          - Avenue
          - Boulevard
        - enum:
          - NW
          - NE
          - SW
          - SE
      items: false
      ```
    Then it should accept:
      ```
      - 1600
      - Pennsylvania
      - Avenue
      - NW
      ```
    # It's ok to not provide all of the items
    And it should accept:
      ```
      - 1600
      - Pennsylvania
      - Avenue
      ```
    # But since `items` is `false`, we can't provide extra items
    But it should NOT accept:
      ```
      - 1600
      - Pennsylvania
      - Avenue
      - NW
      - Washington
      ```

  Scenario: Additional Items with schema
    Given a YAML schema:
      ```
      type: array
      prefixItems:
        - type: number
        - type: string
        - enum:
          - Street
          - Avenue
          - Boulevard
        - enum:
          - NW
          - NE
          - SW
          - SE
      items:
        type: string
      ```
    # Extra string items are ok
    Then it should accept:
      ```
      - 1600
      - Pennsylvania
      - Avenue
      - NW
      - Washington
      ```
    # But not anything else
    But it should NOT accept:
      ```
      - 1600
      - Pennsylvania
      - Avenue
      - NW
      - 20500
      ```

  Scenario: Contains
    # While the items schema must be valid for every item in the array, the `contains` only needs to
    # validate against one or more items in the array.
    Given a YAML schema:
      ```
      type: array
      contains:
        type: number
      ```
    # A single "number" is enough to make this pass
    Then it should accept:
      ```
      - life
      - universe
      - everything
      - 42
      ```
    # But if we have no number, it fails
    But it should NOT accept:
      ```
      - life
      - universe
      - everything
      - forty-two
      ```
    # All numbers is, of course, also ok
    And it should accept:
      ```
      - 1
      - 2
      - 3
      - 4
      - 5
      ```
