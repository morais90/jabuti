(unary_expression "!!" @subject) @concept.error_panic

(catch_block "catch" @subject (block) @_body
  (#match? @_body "^\\{\\s*\\}$")) @concept.error_swallow

(call_expression
  (navigation_expression (identifier) @subject)
  (#eq? @subject "getOrNull")) @concept.error_discard

(annotation
  (constructor_invocation
    (user_type (identifier) @_name)
    (value_arguments) @subject)
  (#eq? @_name "Suppress")) @concept.suppression

(call_expression
  [(identifier) @subject (navigation_expression (identifier) @subject)]
  (#any-of? @subject
    "assertEquals" "assertNotEquals" "assertArrayEquals" "assertIterableEquals"
    "assertLinesMatch" "assertTrue" "assertFalse" "assertNull" "assertNotNull"
    "assertSame" "assertNotSame" "assertThrows" "assertThrowsExactly"
    "assertDoesNotThrow" "assertTimeout" "assertTimeoutPreemptively" "assertAll"
    "assertFails" "assertFailsWith" "assertIs" "assertIsNot" "assertContains"
    "fail" "assertThat")) @concept.assertion

(call_expression (identifier) @call)
(call_expression (navigation_expression) @call)
