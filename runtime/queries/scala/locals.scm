; Scopes

[
  (template_body)
  (function_definition)
  (lambda_expression)
  (for_expression)
  (block)
  (case_clause)
] @local.scope

; Definitions

(function_definition
  name: (identifier) @local.definition.function)

; `def`/method and `class`/constructor parameters; baseline highlight is plain
; `variable`, so the parameter class is what makes these distinct.
(parameter
  name: (identifier) @local.definition.variable.parameter)
(class_parameter
  name: (identifier) @local.definition.variable.parameter)

; Lambda parameters: `(x: Int) => …` (bindings) and bare `x => …`.
(bindings
  (binding
    name: (identifier) @local.definition.variable.parameter))
(lambda_expression
  parameters: (identifier) @local.definition.variable.parameter)

(type_parameters
  name: (identifier) @local.definition.type.parameter)

; Local `val`/`var` bindings; defined so inner references resolve and shadow.
(val_definition
  pattern: (identifier) @local.definition.variable)
(var_definition
  pattern: (identifier) @local.definition.variable)

; References

(identifier) @local.reference

; Member access after `.` is a field/method name, not a local reference.
(field_expression
  field: (identifier) @_)
