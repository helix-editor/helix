; Scopes
;-------

[
 (function_body)
 (function_expression_body)
 (block)
 (for_statement)
 (try_statement)
 (catch_clause)
 (finally_clause)
] @local.scope

; Definitions
;------------

(formal_parameter
 name: (identifier) @local.definition.variable.parameter)

; for-in / C-style loop variable.
(for_loop_parts
 name: (identifier) @local.definition.variable)

(initialized_variable_definition
 name: (identifier) @local.definition.variable)

; References
;------------

(identifier) @local.reference

; Member access selectors carry plain identifiers that are not local references.
(unconditional_assignable_selector
 (identifier) @_)
(conditional_assignable_selector
 (identifier) @_)
