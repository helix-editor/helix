; Scopes

[
  (function_item)
  (closure_expression)
  (block)
] @local.scope

; Definitions

(parameter
  (identifier) @local.definition.variable.parameter)

(closure_parameters (identifier) @local.definition.variable.parameter)

; References
(identifier) @local.reference
