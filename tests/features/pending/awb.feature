@pending
Feature: Generate an air waybill
  Scenario: Render a complete AWB with related records and three goods rows
    Given the populated air waybill example and its ten data tables
    When I generate the AWB PDF
    Then the PDF contains the linked parties and the three goods descriptions
    And the output matches the reference layout
