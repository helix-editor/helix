; Scopes
[
  (class_declaration)
  (function_declaration)
  (lambda_literal)
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

; References
(simple_identifier) @local.reference
(type_identifier) @local.reference
(interpolated_identifier) @local.reference
