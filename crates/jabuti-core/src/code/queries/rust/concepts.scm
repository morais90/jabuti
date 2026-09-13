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

(attribute_item
  (attribute (identifier) @_name arguments: (token_tree) @subject)
  (#eq? @_name "allow")) @concept.suppression

(inner_attribute_item
  (attribute (identifier) @_name arguments: (token_tree) @subject)
  (#eq? @_name "allow")) @concept.suppression

(macro_invocation
  macro: (identifier) @subject
  (#any-of? @subject
    "assert" "assert_eq" "assert_ne"
    "assert_snapshot" "assert_debug_snapshot" "assert_display_snapshot"
    "assert_compact_debug_snapshot" "assert_json_snapshot"
    "assert_compact_json_snapshot" "assert_yaml_snapshot" "assert_ron_snapshot"
    "assert_toml_snapshot" "assert_csv_snapshot" "assert_binary_snapshot")) @concept.assertion

(macro_invocation
  macro: (scoped_identifier name: (identifier) @subject)
  (#any-of? @subject
    "assert" "assert_eq" "assert_ne"
    "assert_snapshot" "assert_debug_snapshot" "assert_display_snapshot"
    "assert_compact_debug_snapshot" "assert_json_snapshot"
    "assert_compact_json_snapshot" "assert_yaml_snapshot" "assert_ron_snapshot"
    "assert_toml_snapshot" "assert_csv_snapshot" "assert_binary_snapshot")) @concept.assertion

(call_expression
  function: (field_expression field: (field_identifier) @subject)
  (#eq? @subject "assert")) @concept.assertion

(call_expression function: (_) @call)
(macro_invocation macro: (_) @call)
