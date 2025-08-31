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

(parameter
  (mutable_specifier)?
  pattern: [
    ; `foo` in `fn x(foo: !) {}`
    (identifier) @local.definition.variable.parameter @variable.parameter
    ; `foo` and `bar` in `fn x((foo, bar): !) {}`
    (tuple_pattern
      [
        (mut_pattern
          (_)
          (identifier))
        (identifier)
      ]*
      [
        (mut_pattern
          (_)
          (identifier) @local.definition.variable.parameter @variable.parameter)
        (identifier) @local.definition.variable.parameter @variable.parameter
      ])
    ; `foo` and `bar` in `fn x(Struct { foo, bar }: !) {}`
    (struct_pattern
      (field_pattern)*
      (field_pattern
        name: (shorthand_field_identifier) @local.definition.variable.parameter @variable.parameter)
    )
    ; `foo` and `bar` in `fn x(TupleStruct(foo, bar): !) {}`
    (tuple_struct_pattern
      type: _
      [
        (mut_pattern
          (_)
          (identifier))
        (identifier)
      ]*
      [
        (mut_pattern
          (_)
          (identifier) @local.definition.variable.parameter @variable.parameter)
        (identifier) @local.definition.variable.parameter @variable.parameter
      ]
    )
    ; `foo` and `bar` in `fn x([foo, bar]: !) {}`
    (slice_pattern
      [
        (mut_pattern
          (_)
          (identifier))
        (identifier)
      ]*
      [
        (mut_pattern
          (_)
          (identifier) @local.definition.variable.parameter @variable.parameter)
        (identifier) @local.definition.variable.parameter @variable.parameter
      ]
    )
  ])


; References
(identifier) @local.reference
