; Scopes

(block) @local.scope
(closure_expression) @local.scope

; Definitions

(parameter
  (identifier) @local.definition)

(closure_parameters (identifier) @local.definition)

; References
(identifier) @local.reference

