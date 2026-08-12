(function_definition
  (function_identifier (lowercase_identifier) @name)) @definition.function

(struct_constructor_declaration
  (lowercase_identifier) @name) @definition.function

(trait_method_declaration
  (function_identifier (lowercase_identifier) @name)) @definition.function

(struct_definition
  (identifier) @name) @definition.struct

(tuple_struct_definition
  (identifier) @name) @definition.struct

(enum_definition
  (identifier) @name) @definition.enum

(type_definition
  (identifier) @name) @definition.type

(error_type_definition
  (identifier) @name) @definition.type

(trait_definition
  (identifier) @name) @definition.interface

(const_definition
  (uppercase_identifier) @name) @definition.constant
