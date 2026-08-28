(function_declaration
  name: (identifier) @name) @definition.function

; `test "name" { ... }`
(test_declaration
  (string (string_content) @name)) @definition.function

; `const Name = struct/enum/union/opaque/error { ... }`
(variable_declaration
  (identifier) @name
  (struct_declaration)) @definition.struct
(variable_declaration
  (identifier) @name
  (enum_declaration)) @definition.enum
(variable_declaration
  (identifier) @name
  (union_declaration)) @definition.struct
(variable_declaration
  (identifier) @name
  (opaque_declaration)) @definition.struct
(variable_declaration
  (identifier) @name
  (error_set_declaration)) @definition.enum

(container_field
  name: (identifier) @name) @definition.field
