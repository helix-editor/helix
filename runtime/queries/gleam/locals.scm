; Scopes
(function_body) @local.scope

(case_clause) @local.scope

; Definitions
(let pattern: (identifier) @local.definition)
(function_parameter name: (identifier) @local.definition)
(list_pattern (identifier) @local.definition)
(list_pattern assign: (identifier) @local.definition)
(tuple_pattern (identifier) @local.definition)
(record_pattern_argument pattern: (identifier) @local.definition)

; References
(identifier) @local.reference
