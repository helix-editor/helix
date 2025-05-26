; Scopes
(function) @local.scope

(case_clause) @local.scope

; Definitions
(function_parameter name: (identifier) @local.definition.variable.parameter)

; References
(identifier) @local.reference
