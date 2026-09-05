(package_header (qualified_identifier) @package)

(import (qualified_identifier) @reference.path)

(source_file (class_declaration (identifier) @declaration))
(source_file (object_declaration (identifier) @declaration))
(source_file (function_declaration (identifier) @declaration))
(source_file (type_alias (identifier) @declaration))

(user_type . (identifier) @reference.name)
(call_expression . (identifier) @reference.name)
(navigation_expression . (identifier) @reference.name)
