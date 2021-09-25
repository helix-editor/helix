; Scopes

(block) @local.scope

; Definitions

(parameter
  (identifier) @local.definition)

(let_declaration
  pattern: (identifier) @local.definition)

(closure_parameters (identifier)) @local.definition

; References
(identifier) @local.reference

