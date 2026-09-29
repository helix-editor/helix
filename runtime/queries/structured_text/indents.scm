; Blocks that should increase the indentation level
[
  ; (program_definition)
  ; (action_definition)
  ; (function_definition)
  ; (method_definition)
  ; (function_block_definition)
  (type_definition)
  (struct_definition)
  (union_definition)
  (enum_definition)
  (var_declaration)
  (var_input_declaration)
  (var_output_declaration)
  (var_in_out_declaration)
  (var_global_declaration)
  (var_temp_declaration)
  (var_stat_declaration)
  (var_external_declaration)
  (var_instance_declaration)
  (var_config_declaration)
  (var_access_declaration)
  (var_generic_constant_declaration)
  (if_statement)
  (case_statement)
  (for_statement)
  (while_statement)
  (repeat_statement)
  (case)
] @indent.begin

; Sibling branches that reset indent relative to the parent block
[
  (elseif_clause)
  (else_clause)
  (else_case)
] @indent.branch

; Tokens that close a block and should outdent
[
  ; "END_PROGRAM"
  ; "END_ACTION"
  ; "END_FUNCTION"
  ; "END_METHOD"
  ; "END_FUNCTION_BLOCK"
  "END_TYPE"
  "END_STRUCT"
  "END_UNION"
  "END_VAR"
  "END_IF"
  "END_CASE"
  "END_FOR"
  "END_WHILE"
  "END_REPEAT"
  "UNTIL"
] @indent.end

; Align multiline expressions and parameter lists
[
  (parenthesis_expression)
  (call_expression)
] @indent.align

