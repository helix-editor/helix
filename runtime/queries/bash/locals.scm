; Scopes

[
  (function_definition)
  (subshell)
] @local.scope

; Definitions

(variable_assignment
  name: (variable_name) @local.definition.variable)

; `local`/`declare`/`export x` etc. bind a bare name.
(declaration_command
  (variable_name) @local.definition.variable)

(for_statement
  variable: (variable_name) @local.definition.variable)

; References

(variable_name) @local.reference
