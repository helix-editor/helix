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

; References
(identifier) @local.reference

; In here, `bar` is a function, as it is equal to a closure:
;
; let bar = || 4;
;
; After this, we know that `bar` must be a function:
;
; let a = bar;
;         ^^^ function
;
; let a = f(bar)
;           ^^^ function
(let_declaration
  pattern: (identifier) @local.definition.function
  value: (closure_expression))
