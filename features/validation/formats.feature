Feature: Format validation

  Scenario: format "date" should accept valid RFC 3339 dates
    Given a YAML schema:
      ```
      type: string
      format: date
      ```
    Then it should accept:
      ```
      "2024-01-15"
      ```
    And it should accept:
      ```
      "2000-02-29"
      ```
    But it should NOT accept:
      ```
      "not-a-date"
      ```
    And it should NOT accept:
      ```
      "2024-13-01"
      ```
    And it should NOT accept:
      ```
      "2023-02-29"
      ```

  Scenario: format "date-time" should accept valid RFC 3339 date-times
    Given a YAML schema:
      ```
      type: string
      format: date-time
      ```
    Then it should accept:
      ```
      "2024-01-15T12:00:00Z"
      ```
    And it should accept:
      ```
      "2024-01-15T12:00:00+05:30"
      ```
    And it should accept:
      ```
      "2024-01-15T12:00:00.123Z"
      ```
    But it should NOT accept:
      ```
      "2024-01-15"
      ```
    And it should NOT accept:
      ```
      "not-a-datetime"
      ```

  Scenario: format "time" should accept valid RFC 3339 times
    Given a YAML schema:
      ```
      type: string
      format: time
      ```
    Then it should accept:
      ```
      "12:00:00Z"
      ```
    And it should accept:
      ```
      "23:59:59+00:00"
      ```
    And it should accept:
      ```
      "12:30:00.123Z"
      ```
    But it should NOT accept:
      ```
      "25:00:00Z"
      ```
    And it should NOT accept:
      ```
      "not-a-time"
      ```

  Scenario: format "duration" should accept valid ISO 8601 durations
    Given a YAML schema:
      ```
      type: string
      format: duration
      ```
    Then it should accept:
      ```
      "P1Y2M3D"
      ```
    And it should accept:
      ```
      "PT1H30M"
      ```
    But it should NOT accept:
      ```
      "not-a-duration"
      ```
    And it should NOT accept:
      ```
      "P"
      ```

  Scenario: format "email" should accept valid email addresses
    Given a YAML schema:
      ```
      type: string
      format: email
      ```
    Then it should accept:
      ```
      "user@example.com"
      ```
    And it should accept:
      ```
      "user+tag@sub.example.com"
      ```
    But it should NOT accept:
      ```
      "not-an-email"
      ```
    And it should NOT accept:
      ```
      "@example.com"
      ```

  Scenario: format "hostname" should accept valid RFC 1123 hostnames
    Given a YAML schema:
      ```
      type: string
      format: hostname
      ```
    Then it should accept:
      ```
      "example.com"
      ```
    And it should accept:
      ```
      "localhost"
      ```
    But it should NOT accept:
      ```
      "-invalid.com"
      ```
    And it should NOT accept:
      ```
      ""
      ```

  Scenario: format "ipv4" should accept valid IPv4 addresses
    Given a YAML schema:
      ```
      type: string
      format: ipv4
      ```
    Then it should accept:
      ```
      "192.168.1.1"
      ```
    And it should accept:
      ```
      "0.0.0.0"
      ```
    But it should NOT accept:
      ```
      "999.999.999.999"
      ```
    And it should NOT accept:
      ```
      "not-an-ip"
      ```

  Scenario: format "ipv6" should accept valid IPv6 addresses
    Given a YAML schema:
      ```
      type: string
      format: ipv6
      ```
    Then it should accept:
      ```
      "::1"
      ```
    And it should accept:
      ```
      "2001:db8::1"
      ```
    But it should NOT accept:
      ```
      "not-ipv6"
      ```
    And it should NOT accept:
      ```
      "192.168.1.1"
      ```

  Scenario: format "uri" should accept valid URIs
    Given a YAML schema:
      ```
      type: string
      format: uri
      ```
    Then it should accept:
      ```
      "https://example.com"
      ```
    And it should accept:
      ```
      "urn:isbn:0451450523"
      ```
    But it should NOT accept:
      ```
      "not a uri"
      ```

  Scenario: format "uuid" should accept valid UUIDs
    Given a YAML schema:
      ```
      type: string
      format: uuid
      ```
    Then it should accept:
      ```
      "550e8400-e29b-41d4-a716-446655440000"
      ```
    But it should NOT accept:
      ```
      "not-a-uuid"
      ```
    And it should NOT accept:
      ```
      "550e8400e29b41d4a716446655440000"
      ```

  Scenario: format "json-pointer" should accept valid JSON Pointers
    Given a YAML schema:
      ```
      type: string
      format: json-pointer
      ```
    Then it should accept:
      ```
      ""
      ```
    And it should accept:
      ```
      "/foo/bar"
      ```
    And it should accept:
      ```
      "/foo/0"
      ```
    But it should NOT accept:
      ```
      "no-leading-slash"
      ```

  Scenario: format "regex" should accept valid regular expressions
    Given a YAML schema:
      ```
      type: string
      format: regex
      ```
    Then it should accept:
      ```
      "^[a-z]+$"
      ```
    And it should accept:
      ```
      ".*"
      ```
    But it should NOT accept:
      ```
      "[invalid"
      ```

  Scenario: unknown format should be annotation-only and always accept
    Given a YAML schema:
      ```
      type: string
      format: my-custom-format
      ```
    Then it should accept:
      ```
      "anything goes"
      ```
    And it should accept:
      ```
      "12345"
      ```

  Scenario: format with other string constraints
    Given a YAML schema:
      ```
      type: string
      format: email
      minLength: 10
      ```
    Then it should accept:
      ```
      "user@example.com"
      ```
    But it should NOT accept:
      ```
      "a@b.c"
      ```
    And it should NOT accept:
      ```
      "not-an-email-at-all"
      ```
