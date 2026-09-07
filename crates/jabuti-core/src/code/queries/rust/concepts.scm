(call_expression
  function: (field_expression field: (field_identifier) @subject)
  (#any-of? @subject "unwrap" "expect")) @concept.error_panic

(call_expression
  function: (field_expression field: (field_identifier) @subject)
  (#eq? @subject "ok")) @concept.error_discard

(let_declaration pattern: "_" @subject) @concept.error_discard

(match_arm
  pattern: (match_pattern (tuple_struct_pattern type: (identifier) @subject))
  value: (block) @_body
  (#eq? @subject "Err")
  (#match? @_body "^\\{\\s*\\}$")) @concept.error_swallow

(call_expression function: (_) @call)
