; Scopes

[
  (function_item)
  (struct_item)
  (enum_item)
  (union_item)
  (type_item)
  (trait_item)
  (impl_item)
  (closure_expression)
  (block)
] @local.scope

; Definitions

(function_item
  (parameters
    (parameter
      pattern: (identifier) @local.definition.variable.parameter)))

(closure_parameters (identifier) @local.definition.variable.parameter)

; References
(identifier) @local.reference
