; Scopes

(function_body) @local.scope

; Definitions

(argument
  (value (var (local_var) @local.definition)))

(instruction
  (local_var) @local.definition)

; References
(local_var) @local.reference
