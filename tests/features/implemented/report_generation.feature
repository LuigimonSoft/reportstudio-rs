Feature: Generate a report PDF
  The application supplies the report definition and table data.

  Scenario: Generate a PDF in memory using supplied detail records
    Given a simple report definition
    And a details table with two records
    When I generate the PDF
    Then the PDF contains both detail records

  Scenario: Save a generated PDF to a path
    Given a simple report definition
    And a details table with two records
    When I save the PDF
    Then the saved file contains both detail records

  Scenario: Reject generation when the required table is missing
    Given a simple report definition
    When I generate the PDF
    Then a missing data error is returned

  Scenario: Reject a malformed report file
    Given a malformed report definition
    When I generate the PDF
    Then a JSON error is returned

  Scenario: Reject invalid rows without changing the table
    Given a details table with one text column
    When I add a row with the wrong type
    Then the row is rejected and the table remains empty

  Scenario: Resolve a related field for each detail record
    Given a report with a relationship to customers
    And customer and detail records with matching keys
    When I generate the PDF
    Then the PDF contains the matching customer and detail

  Scenario: Render a heading containing two lines
    Given a report with a multiline heading
    And a details table with two records
    When I generate the PDF
    Then the PDF contains both lines of the heading

  Scenario: Render detail records inside an editable table
    Given a report with a detail table
    And a details table with two records
    When I generate the PDF
    Then the PDF contains both detail records
