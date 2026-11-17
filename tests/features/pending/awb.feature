@pending
Feature: Air waybill with overflowing merchandise
  Scenario: Continue all merchandise rows onto subsequent pages
    Given an air waybill with more merchandise than fits on one page
    When I generate the AWB PDF
    Then every merchandise row appears exactly once
    And the page headers and footers are placed correctly
