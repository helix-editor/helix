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
  pattern: (identifier) @local.definition.variable.parameter)

(closure_parameters (identifier) @local.definition.variable.parameter)

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
