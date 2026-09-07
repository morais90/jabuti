(import_statement
  (import_clause
    (named_imports
      (import_specifier
        name: (_) @import.name
        alias: (_)? @import.alias)))
  source: (string) @import.module)

(import_statement
  (import_clause (namespace_import (identifier) @import.alias))
  source: (string) @import.module)

(import_statement
  (import_clause . (identifier) @import.alias)
  source: (string) @import.module)

(import_statement source: (string) @reference.module)
(export_statement source: (string) @reference.module)

(function_declaration name: (identifier) @declaration)
(generator_function_declaration name: (identifier) @declaration)
(class_declaration name: (type_identifier) @declaration)
(abstract_class_declaration name: (type_identifier) @declaration)
(interface_declaration name: (type_identifier) @declaration)
(enum_declaration name: (identifier) @declaration)
(type_alias_declaration name: (type_identifier) @declaration)
(method_definition name: (_) @declaration)
(method_signature name: (_) @declaration)
(abstract_method_signature name: (_) @declaration)
(export_statement
  declaration: (lexical_declaration
    (variable_declarator name: (identifier) @declaration)))
(export_statement
  declaration: (variable_declaration
    (variable_declarator name: (identifier) @declaration)))
(public_field_definition name: (_) @declaration)


(identifier) @reference.mention
(type_identifier) @reference.mention
(property_identifier) @reference.mention
