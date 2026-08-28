[
  (struct_definition)
  (macro_definition)
  (function_definition)
  (compound_statement)
  (if_statement)
  (try_statement)
  (for_statement)
  (while_statement)
  (let_statement)
  (quote_statement)
  (do_clause)
  (assignment)
  (for_binding)
  (call_expression)
  (parenthesized_expression)
  (tuple_expression)
  (comprehension_expression)
  (matrix_expression)
  (vector_expression)
] @indent

[
  "end"
  ")"
  "]"
  "}"
  "else"
  "elseif"
  "catch"
  "finally"
] @outdent

(argument_list
  . (_) @anchor
  ) @align

(curly_expression
  . (_) @anchor
  ) @align
