(unary_expression "!!" @construct) @mask.panic

(catch_block "catch" @construct (block) @_body
  (#match? @_body "^\\{\\s*\\}$")) @mask.swallow

(call_expression
  (navigation_expression (identifier) @construct)
  (#eq? @construct "getOrNull")) @mask.discard
