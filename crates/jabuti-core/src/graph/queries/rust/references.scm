(scoped_use_list) @reference.list

(scoped_identifier path: (identifier)) @reference.path
(scoped_identifier path: (crate)) @reference.path
(scoped_identifier path: (super)) @reference.path
(scoped_identifier path: (self)) @reference.path

(token_tree (crate) @reference.token)
(token_tree (super) @reference.token)
(token_tree (self) @reference.token)
(token_tree (identifier) @reference.token)

(function_item name: (identifier) @declaration)
(struct_item name: (type_identifier) @declaration)
(enum_item name: (type_identifier) @declaration)
(union_item name: (type_identifier) @declaration)
(trait_item name: (type_identifier) @declaration)
(type_item name: (type_identifier) @declaration)
(const_item name: (identifier) @declaration)
(static_item name: (identifier) @declaration)

(mod_item (visibility_modifier) name: (identifier) @export.module)
(use_declaration (visibility_modifier) argument: (_) @export.use)

(identifier) @reference.mention
(type_identifier) @reference.mention
(field_identifier) @reference.mention
