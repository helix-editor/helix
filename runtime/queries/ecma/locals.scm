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

(pattern/identifier) @local.definition

(pattern/rest_pattern 
  (identifier) @local.definition)
  
(arrow_function
  parameter: (identifier) @local.definition)

(variable_declarator
  name: (identifier) @local.definition)

; References
;------------

(identifier) @local.reference
