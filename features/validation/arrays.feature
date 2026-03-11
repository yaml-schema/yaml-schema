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

  Scenario: minContains
    # minContains requires at least N items to match the contains schema
    Given a YAML schema:
      ```
      type: array
      contains:
        type: number
      minContains: 2
      ```
    # Two numbers satisfies minContains: 2
    Then it should accept:
      ```
      - apple
      - 1
      - banana
      - 2
      ```
    # Only one number is not enough
    But it should NOT accept:
      ```
      - apple
      - 1
      - banana
      ```
    # All numbers also satisfies the constraint
    And it should accept:
      ```
      - 1
      - 2
      - 3
      ```

  Scenario: maxContains
    # maxContains requires at most N items to match the contains schema
    Given a YAML schema:
      ```
      type: array
      contains:
        type: number
      maxContains: 3
      ```
    # Three numbers is within the limit
    Then it should accept:
      ```
      - 1
      - apple
      - 2
      - banana
      - 3
      ```
    # Four numbers exceeds maxContains: 3
    But it should NOT accept:
      ```
      - 1
      - 2
      - 3
      - 4
      ```
    # One number is fine (still at least 1 by default minContains)
    And it should accept:
      ```
      - apple
      - 1
      - banana
      ```

  Scenario: minContains and maxContains together
    Given a YAML schema:
      ```
      type: array
      contains:
        type: number
      minContains: 2
      maxContains: 3
      ```
    # Exactly 2 matches — valid
    Then it should accept:
      ```
      - apple
      - 1
      - 2
      - banana
      ```
    # Exactly 3 matches — valid
    And it should accept:
      ```
      - 1
      - apple
      - 2
      - 3
      ```
    # Only 1 match — below minContains
    But it should NOT accept:
      ```
      - apple
      - 1
      - banana
      - cherry
      ```
    # 4 matches — exceeds maxContains
    And it should NOT accept:
      ```
      - 1
      - 2
      - 3
      - 4
      ```

  Scenario: minContains of 0
    # Setting minContains to 0 with contains means "contains is satisfied even if nothing matches"
    Given a YAML schema:
      ```
      type: array
      contains:
        type: number
      minContains: 0
      ```
    # No numbers at all — still valid because minContains is 0
    Then it should accept:
      ```
      - apple
      - banana
      - cherry
      ```
    # Having numbers is also fine
    And it should accept:
      ```
      - apple
      - 1
      - banana
      ```
