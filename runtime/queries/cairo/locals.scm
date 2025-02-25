; Scopes

[
  (function_item)
  (struct_item)
  (enum_item)
  (type_item)
  (trait_item)
  (impl_item)
  (closure_expression)
  (block)
] @local.scope

; Definitions

(parameter
  (identifier) @local.definition)

(type_parameters
  (type_identifier) @local.definition)
(constrained_type_parameter
  left: (type_identifier) @local.definition)

(closure_parameters (identifier) @local.definition)

; References
(identifier) @local.reference
(type_identifier) @local.reference
