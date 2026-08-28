; Scopes
;-------

[
  (statement_block)
  (arrow_function)
  (function_expression)
  (function_declaration)
  (method_definition)
  (for_statement)
  (for_in_statement)
  (catch_clause)
  (finally_clause)
] @local.scope

; Definitions
;------------

; i => ...
(arrow_function
  parameter: (identifier) @local.definition.variable.parameter)

; References
;------------

(identifier) @local.reference
