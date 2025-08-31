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

; Mutable variables

[
  (let_declaration
    (mutable_specifier)
    pattern: (identifier) @local.definition.variable.mutable @variable.mutable)
  (parameter
    (mutable_specifier)
    pattern: (identifier) @local.definition.variable.mutable @variable.mutable)
  (mut_pattern
    (mutable_specifier)
    (identifier) @local.definition.variable.mutable @variable.mutable)
]

; References
(identifier) @local.reference
