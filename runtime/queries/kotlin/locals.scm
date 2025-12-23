; Scopes
[
  (class_declaration)
  (function_declaration)
] @local.scope

; Definitions
(type_parameter
  (type_identifier) @local.definition.type.parameter)

(parameter
  (simple_identifier) @local.definition.variable.parameter)

; References
(simple_identifier) @local.reference
(type_identifier) @local.reference
