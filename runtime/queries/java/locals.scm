; Scopes

[
  (method_declaration)
  (constructor_declaration)
  (compact_constructor_declaration)
  (lambda_expression)
  (for_statement)
  (enhanced_for_statement)
  (block)
] @local.scope

; Definitions

(formal_parameter
  name: (identifier) @local.definition.variable.parameter)
; `Type... name` — the name lives in a variable_declarator child, not a `name:` field.
(spread_parameter
  (variable_declarator
    name: (identifier) @local.definition.variable.parameter))
(catch_formal_parameter
  name: (identifier) @local.definition.variable.parameter)
; Lambda params: `x -> …` (single bare identifier) and `(x, y) -> …` (inferred).
(lambda_expression
  parameters: (identifier) @local.definition.variable.parameter)
(inferred_parameters
  (identifier) @local.definition.variable.parameter)

(local_variable_declaration
  (variable_declarator
    name: (identifier) @local.definition.variable))
(enhanced_for_statement
  name: (identifier) @local.definition.variable)

; References

(identifier) @local.reference

; Discards: identifiers that look like references but aren't variables.

; `obj.method(…)` — the method name is not a variable.
(method_invocation
  name: (identifier) @_)
; `obj.field` — the field name is not a local.
(field_access
  field: (identifier) @_)
; `Outer.name` qualified access.
(scoped_identifier
  name: (identifier) @_)
