(call_expression
  function: (field_expression field: (field_identifier) @construct)
  (#any-of? @construct "unwrap" "expect")) @mask.panic

(call_expression
  function: (field_expression field: (field_identifier) @construct)
  (#eq? @construct "ok")) @mask.discard

(let_declaration pattern: "_" @construct) @mask.discard

(match_arm
  pattern: (match_pattern (tuple_struct_pattern type: (identifier) @construct))
  value: (block) @_body
  (#eq? @construct "Err")
  (#match? @_body "^\\{\\s*\\}$")) @mask.swallow
