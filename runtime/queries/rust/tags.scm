(struct_item
  name: (type_identifier) @definition.struct)

(const_item
  name: (identifier) @definition.constant)

(trait_item
  name: (type_identifier) @definition.interface)

(function_item
  name: (identifier) @definition.function)

(function_signature_item
  name: (identifier) @definition.function)

(enum_item
  name: (type_identifier) @definition.type)

(enum_variant
  name: (identifier) @definition.struct)

(mod_item
  name: (identifier) @definition.module)

(macro_definition
  name: (identifier) @definition.macro)
