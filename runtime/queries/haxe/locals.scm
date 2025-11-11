; Scopes
[
 (block)
 (function_declaration)
] @local.scope

; Definitions
(function_arg name: (identifier) @local.definition.variable.parameter)
(variable_declaration name: (identifier) @local.definition.variable)

; References
(block (identifier)) @local.reference
