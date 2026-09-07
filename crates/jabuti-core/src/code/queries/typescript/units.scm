(module name: (_) @name) @unit.module
(internal_module name: (_) @name) @unit.module

(class_declaration name: (_) @name) @unit.type
(abstract_class_declaration name: (_) @name) @unit.type
(interface_declaration name: (_) @name) @unit.type
(enum_declaration name: (_) @name) @unit.type
(type_alias_declaration name: (_) @name) @unit.type

(function_declaration
  name: (_) @name
  parameters: (_) @parameters) @unit.function

(generator_function_declaration
  name: (_) @name
  parameters: (_) @parameters) @unit.function

(function_signature
  name: (_) @name
  parameters: (_) @parameters) @unit.function

(method_definition
  name: (_) @name
  parameters: (_) @parameters) @unit.function

(method_signature
  name: (_) @name
  parameters: (_) @parameters) @unit.function

(abstract_method_signature
  name: (_) @name
  parameters: (_) @parameters) @unit.function

(function_expression
  parameters: (_) @parameters) @unit.closure

(variable_declarator
  name: (_) @name
  value: (arrow_function
    parameters: (_) @parameters) @unit.function)

(variable_declarator
  name: (_) @name
  value: (arrow_function
    parameter: (_) @parameters) @unit.function)

(variable_declarator
  name: (_) @name
  value: (function_expression
    parameters: (_) @parameters) @unit.function)

(variable_declarator
  name: (_) @name
  value: (generator_function
    parameters: (_) @parameters) @unit.function)

(public_field_definition
  name: (_) @name
  value: (arrow_function
    parameters: (_) @parameters) @unit.function)

(public_field_definition
  name: (_) @name
  value: (arrow_function
    parameter: (_) @parameters) @unit.function)

(pair
  key: (_) @name
  value: (arrow_function
    parameters: (_) @parameters) @unit.function)

(pair
  key: (_) @name
  value: (arrow_function
    parameter: (_) @parameters) @unit.function)

(generator_function
  parameters: (_) @parameters) @unit.closure

(arrow_function
  parameters: (_) @parameters) @unit.closure

(arrow_function
  parameter: (_) @parameters) @unit.closure
