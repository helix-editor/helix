(struct_item
  name: (type_identifier) @definition.struct
  body: (field_declaration_list))

(const_item
  name: (identifier) @definition.constant)

(trait_item
  name: (type_identifier) @definition.interface
  body: (declaration_list))

(function_item
  name: (identifier) @definition.function
  parameters: (parameters)
  body: (block))

(function_signature_item
  name: (identifier) @definition.function
  parameters: (parameters))

(enum_item
  name: (type_identifier) @definition.type
  body: (enum_variant_list))

(enum_variant
  name: (identifier) @definition.struct)

(mod_item
  name: (identifier) @definition.module
  body: (declaration_list))

(macro_invocation
  macro: (identifier) @definition.macro)
