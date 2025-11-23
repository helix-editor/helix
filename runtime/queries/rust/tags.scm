(struct_item
  name: (type_identifier) @name) @definition.struct

(const_item
  name: (identifier) @name) @definition.constant

(trait_item
  name: (type_identifier) @name) @definition.interface

(function_item
  name: (identifier) @name) @definition.function

(function_signature_item
  name: (identifier) @name) @definition.function

(enum_item
  name: (type_identifier) @name) @definition.type

(enum_variant
  name: (identifier) @name) @definition.struct

(type_item
  name: (type_identifier) @name) @definition.type

(mod_item
  name: (identifier) @name) @definition.module

(macro_definition
  name: (identifier) @name) @definition.macro
