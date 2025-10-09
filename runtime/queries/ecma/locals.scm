; Scopes
;-------

[
  (statement_block)
  (arrow_function)
  (function_expression)
  (function_declaration)
  (method_definition)
] @local.scope

; Definitions
;------------

; i => ...
(arrow_function
  parameter: (identifier) @local.definition.variable.parameter)

; References
;------------

(identifier) @local.reference
