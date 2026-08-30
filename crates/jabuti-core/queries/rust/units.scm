(mod_item name: (identifier) @name) @unit.module

(struct_item name: (type_identifier) @name) @unit.type
(enum_item name: (type_identifier) @name) @unit.type
(union_item name: (type_identifier) @name) @unit.type
(trait_item name: (type_identifier) @name) @unit.type
(impl_item type: (_) @name) @unit.type

(function_item
  name: (identifier) @name
  parameters: (parameters) @parameters) @unit.function

(closure_expression
  parameters: (closure_parameters) @parameters) @unit.closure
