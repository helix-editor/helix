(function_declaration
  name: (identifier) @name) @definition.function

(struct_declaration
  name: (type_identifier) @name) @definition.struct

(enum_declaration
  name: (type_identifier) @name) @definition.enum

(variant
  name: (identifier) @name) @definition.struct
