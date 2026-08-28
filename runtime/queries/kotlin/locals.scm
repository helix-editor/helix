; Scopes
[
  (class_declaration)
  (function_declaration)
  (lambda_literal)
  ; `fun(x) { … }` expression form: has its own parameters and body.
  (anonymous_function)
  (control_structure_body)
  (when_entry)
  ; for/while loop variables are declared on the statement, not in its body.
  (for_statement)
] @local.scope

; Definitions
(type_parameter
  (type_identifier) @local.definition.type.parameter)

(parameter
  (simple_identifier) @local.definition.variable.parameter)

(lambda_literal
  (lambda_parameters
    (variable_declaration
      (simple_identifier) @local.definition.variable.parameter)))

; Loop and local `val`/`var` bindings; defined so inner references resolve and
; shadow correctly.
(variable_declaration
  (simple_identifier) @local.definition.variable)

; References
(simple_identifier) @local.reference
(type_identifier) @local.reference
(interpolated_identifier) @local.reference

; Member access after `.` is not a local reference.
(navigation_suffix
  (simple_identifier) @_)
