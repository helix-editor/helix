; Scopes

(function_definition) @local.scope

; Definitions

; `set`/`read` bindings aren't a dedicated node (the name is an untyped `word`
; argument), so only the for-loop variable is reliably a definition.
(for_statement
  variable: (variable_name) @local.definition.variable)

; References

(variable_name) @local.reference
