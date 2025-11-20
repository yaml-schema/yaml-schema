Feature: String validation

  Scenario: "type: string" should accept strings
    Given a YAML schema:
      ```
      type: string
      ```
    Then it should accept:
      ```
      "Déjà vu"
      ```
    And it should accept:
      ```
      ""
      ```
    And it should accept:
      ```
      "42"
      ```
    But it should NOT accept:
      ```
      42
      ```

  Scenario: length validation
    Given a YAML schema:
      ```
      type: string
      minLength: 2
      maxLength: 3
      ```
    Then it should NOT accept:
      ```
      "A"
      ```
    But it should accept:
      ```
      "AB"
      ```
    And it should accept:
      ```
      "ABC"
      ```
    But it should NOT accept:
      ```
      "ABCD"
      ```

  Scenario: pattern validation
    Given a YAML schema:
      ```
      type: string
      pattern: "^(\\([0-9]{3}\\))?[0-9]{3}-[0-9]{4}$"
      ```
    Then it should accept:
      ```
      "555-1212"
      ```
    And it should accept:
      ```
      "(888)555-1212"
      ```
    But it should NOT accept:
      ```
      "(888)555-1212 ext. 532"
      ```
    And it should NOT accept:
      ```
      "(800)FLOWERS"
      ```

  Scenario: string with description
    Given a YAML schema:
      ```
      type: string
      description: "First name"
      ```
    Then it should accept:
      ```
      "John Doe"
      ```
    And it should accept:
      ```
      "Marie"
      ```
    But it should NOT accept:
      ```
      1
      ```
    And it should NOT accept:
      ```
      true
      ```
