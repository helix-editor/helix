; Scopes

[
  (function_definition)
  (let_expression)
] @local.scope

; Definitions

; `function_parameter` is `name: (identifier) type: (parameter_type)`.
(function_parameter
  (identifier) @local.definition.variable.parameter)

; `local_binding` in a `let` is `name: (identifier) value: (_)`; anchor the name.
(let_expression
  (local_binding
    . (identifier) @local.definition.variable))

; References

(identifier) @local.reference

; The function name in a signature is a definition site, not a reference.
(function_signature
  (identifier) @_)

; Operators in call forms are function names, not variable references.
(contract_function_call
  operator: (identifier) @_)

; Tuple keys are field names, not variable references.
(tuple_lit
  key: (identifier) @_)
(tuple_type
  key: (identifier) @_)
