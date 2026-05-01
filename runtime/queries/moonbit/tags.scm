(function_definition
  (function_identifier) @definition.function)

(struct_constructor_declaration
  (lowercase_identifier) @definition.function)

(trait_method_declaration
  (function_identifier) @definition.function)

(struct_definition
  (identifier) @definition.struct)

(enum_definition
  (identifier) @definition.enum)

(type_definition
  (identifier) @definition.type)

(error_type_definition
  (identifier) @definition.type)

(trait_definition
  (identifier) @definition.interface)

(const_definition
  (uppercase_identifier) @definition.constant)
