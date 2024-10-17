(value_declaration) @local.scope
(type_alias_declaration) @local.scope
(type_declaration) @local.scope
(type_annotation) @local.scope
(port_annotation) @local.scope
(infix_declaration) @local.scope
(let_in_expr) @local.scope

(function_declaration_left (lower_pattern (lower_case_identifier)) @local.definition)
(function_declaration_left (lower_case_identifier) @local.definition)

(value_expr(value_qid(upper_case_identifier)) @local.reference)
(value_expr(value_qid(lower_case_identifier)) @local.reference)
(type_ref (upper_case_qid) @local.reference)
