(unary_expression "!!" @subject) @concept.error_panic

(catch_block "catch" @subject (block) @_body
  (#match? @_body "^\\{\\s*\\}$")) @concept.error_swallow

(call_expression
  (navigation_expression (identifier) @subject)
  (#eq? @subject "getOrNull")) @concept.error_discard

(call_expression (identifier) @call)
(call_expression (navigation_expression) @call)
