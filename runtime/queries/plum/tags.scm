(function_declaration
  name: (identifier) @name) @definition.function

(method_declaration
  name: (identifier) @name) @definition.function

(tree_declaration
  name: (type_identifier) @name) @definition.class

(trait_declaration
  name: (type_identifier) @name) @definition.interface

(kind_declaration
  name: (type_identifier) @name) @definition.enum

(field_declaration
  name: (identifier) @name) @definition.field

(constant_declaration
  name: (identifier) @name) @definition.constant
