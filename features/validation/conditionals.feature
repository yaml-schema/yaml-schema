Feature: Conditional schemas

  # See https://json-schema.org/understanding-json-schema/reference/conditionals#ifthenelse
  Scenario: dependentRequired
    Given a YAML schema:
      ```
      type: object
      properties:
        name:
          type: string
        credit_card:
          type: number
        billing_address:
          type: string
      required:
        - name
      dependentRequired:
        credit_card:
          - billing_address
      ```
    Then it should accept:
      ```
      name: John Doe
      credit_card: 5555555555555555
      billing_address: 555 Debtor's Lane
      ```
    # This instance has a credit_card, but it's missing a billing_address
    But it should NOT accept:
      ```
      name: John Doe
      credit_card: 5555555555555555
      ```
    # This is okay, since we have neither a credit_card, or a billing_address
    But it should accept:
      ```
      name: John Doe
      ```
    # Note that dependencies are not bidirectional. It's okay to have a billing address without a credit card number.
    And it should accept:
      ```
      name: John Doe
      billing_address: 555 Debtor's Lane
      ```
    
  # To fix the last issue above (that dependencies are not bidirectional), you can, of course, define the bidirectional dependencies explicitly
  Scenario: bidirectional dependentRequired
    Given a YAML schema:
      ```
      type: object
      properties:
        name:
          type: string
        credit_card:
          type: number
        billing_address:
          type: string
      required:
        - name
      dependentRequired:
        credit_card:
          - billing_address
        billing_address:
          - credit_card
      ```
    Then it should accept:
      ```
      name: John Doe
      credit_card: 5555555555555555
      billing_address: 555 Debtor's Lane
      ```
    # This instance has a credit_card, but it's missing a billing_address
    But it should NOT accept:
      ```
      name: John Doe
      credit_card: 5555555555555555
      ```
    # This has a billing_address, but is missing a credit_card
    But it should NOT accept:
      ```
      name: John Doe
      billing_address: 555 Debtor's Lane
      ```
    # This is okay, since we have neither a credit_card, or a billing_address
    But it should accept:
      ```
      name: John Doe
      ```

  Scenario: dependentSchemas
    Given a YAML schema:
      ```
      type: object
      properties:
        name:
          type: string
        credit_card:
          type: number
      required:
        - name
      dependentSchemas:
        credit_card:
          properties:
            billing_address:
              type: string
          required:
            - billing_address
      ```
    Then it should accept:
      ```
      name: John Doe
      credit_card: 5555555555555555
      billing_address: 555 Debtor's Lane
      ```
    # This instance has a credit_card, but it's missing a billing_address
    But it should NOT accept:
      ```
      name: John Doe
      credit_card: 5555555555555555
      ```
    # This is okay, since we have neither a credit_card, or a billing_address
    But it should accept:
      ```
      name: John Doe
      billing_address: 555 Debtor's Lane
      ```