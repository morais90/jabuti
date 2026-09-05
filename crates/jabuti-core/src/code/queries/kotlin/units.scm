(class_declaration name: (identifier) @name) @unit.type
(object_declaration name: (identifier) @name) @unit.type

(function_declaration
  name: (identifier) @name
  (function_value_parameters) @parameters) @unit.function

(lambda_literal) @unit.closure
