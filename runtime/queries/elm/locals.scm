; Scopes
(value_declaration) @local.scope
(type_alias_declaration) @local.scope
(type_declaration) @local.scope
(type_annotation) @local.scope
(port_annotation) @local.scope
(infix_declaration) @local.scope
(let_in_expr) @local.scope
(anonymous_function_expr) @local.scope
(case_of_branch) @local.scope

; Definitions
; The lower_case_identifier child (not the pattern field) is the function name.
(function_declaration_left
  (lower_case_identifier) @local.definition.function)
; `lower_pattern` is the leaf binding node in every pattern position (function
; params, lambda params, case branches, let/top-level destructuring) at any
; nesting depth, so a single rule covers names inside tuple/list/cons/record
; patterns too.
(lower_pattern (lower_case_identifier) @local.definition.variable)

; Function and lambda parameters: re-capture with the parameter class. Placed
; after the broad `variable` rule so parameters win the `variable.parameter`
; highlight.
(function_declaration_left
  pattern: (_ (lower_case_identifier) @local.definition.variable.parameter))
(anonymous_function_expr
  param: (pattern (_ (lower_case_identifier)) @local.definition.variable.parameter))

; References
(value_expr (value_qid (upper_case_identifier)) @local.reference)
(value_expr (value_qid (lower_case_identifier)) @local.reference)
(type_ref (upper_case_qid) @local.reference)
