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

; Parameters

(parameter
  pattern: [
    ; `foo` in `fn x(foo: !) {}`
    (identifier) @local.definition.variable.parameter @variable.parameter
    ; `foo` and `bar` in `fn x((foo, bar): !) {}`
    (tuple_pattern (identifier)* (identifier) @local.definition.variable.parameter @variable.parameter)
    ; `foo` and `bar` in `fn x(Struct { foo, bar }: !) {}`
    (struct_pattern
      (field_pattern)*
      (field_pattern
        name: (shorthand_field_identifier) @local.definition.variable.parameter @variable.parameter)
    )
    ; `foo` and `bar` in `fn x(TupleStruct(foo, bar): !) {}`
    (tuple_struct_pattern
      type: _
      (identifier)*
      (identifier) @local.definition.variable.parameter @variable.parameter
    )
    ; `foo` and `bar` in `fn x([foo, bar]: !) {}`
    (slice_pattern
      (identifier)*
      (identifier) @local.definition.variable.parameter @variable.parameter
    )
  ])

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
