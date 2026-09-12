(catch_clause
  "catch" @subject
  body: (statement_block) @_body
  (#match? @_body "^\\{\\s*\\}$")) @concept.error_swallow

(call_expression
  function: (member_expression
    property: (property_identifier) @subject)
  arguments: (arguments
    [
      (arrow_function body: (statement_block) @_body)
      (function_expression body: (statement_block) @_body)
    ])
  (#eq? @subject "catch")
  (#match? @_body "^\\{\\s*\\}$")) @concept.error_swallow

((comment) @subject
  (#match? @subject "^//\\s*@ts-ignore")) @concept.suppression

((comment) @subject
  (#match? @subject "^//\\s*@ts-nocheck")) @concept.suppression

((comment) @subject
  (#match? @subject "^(//|/\\*)\\s*eslint-disable")) @concept.suppression

(as_expression (predefined_type) @subject
  (#eq? @subject "any")) @concept.suppression

(type_assertion (type_arguments (predefined_type) @subject)
  (#eq? @subject "any")) @concept.suppression

(call_expression function: (_) @call)
