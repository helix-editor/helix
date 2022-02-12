[
  (use_list)
  (block)
  (match_block)
  (arguments)
  (parameters)
  (declaration_list)
  (field_declaration_list)
  (field_initializer_list)
  (struct_pattern)
  (tuple_pattern)
  (unit_expression)
  (enum_variant_list)
  (call_expression)
  (binary_expression)
  (field_expression)
  (tuple_expression)
  (array_expression)
  (where_clause)
  (macro_invocation)
] @indent

[
  "where"
  "}"
  "]"
  ")"
] @outdent

(assignment_expression
  right: (_) @indent
  (#set! "scope" "all"))
(compound_assignment_expr
  right: (_) @indent
  (#set! "scope" "all"))
(let_declaration
  value: (_) @indent
  (#set! "scope" "all"))