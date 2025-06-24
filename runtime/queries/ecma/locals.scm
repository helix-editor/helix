; Scopes
;-------

[
  (statement_block)
  (function)
  (arrow_function)
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
