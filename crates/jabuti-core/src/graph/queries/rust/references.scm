(scoped_use_list) @reference.list

(scoped_identifier path: (identifier)) @reference.path
(scoped_identifier path: (crate)) @reference.path
(scoped_identifier path: (super)) @reference.path
(scoped_identifier path: (self)) @reference.path

(token_tree (crate) @reference.token)
(token_tree (super) @reference.token)
(token_tree (self) @reference.token)
(token_tree (identifier) @reference.token)
