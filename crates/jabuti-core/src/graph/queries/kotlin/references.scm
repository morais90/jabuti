(package_header (qualified_identifier) @package)

(import (qualified_identifier) @reference.path)

(source_file (class_declaration name: (identifier) @declaration))
(source_file (object_declaration name: (identifier) @declaration))
(source_file (function_declaration name: (identifier) @declaration))
(source_file (type_alias type: (identifier) @declaration))

(user_type . (identifier) @reference.name)
(call_expression . (identifier) @reference.name)
(navigation_expression . (identifier) @reference.name)

(identifier) @reference.mention
