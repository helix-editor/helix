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

  (token_tree)
  (macro_definition)
  (token_tree_pattern)
  (token_repetition)
] @indent

[
  "}"
  "]"
  ")"
] @outdent

; Indent the right side of assignments.
; The #not-same-line? predicate is required to prevent an extra indent for e.g.
; an else-clause where the previous if-clause starts on the same line as the assignment.
(assignment_expression
  .
  (_) @expr-start
  right: (_) @indent
  (#not-same-line? @indent @expr-start)
  (#set! "scope" "all")
)
(compound_assignment_expr
  .
  (_) @expr-start
  right: (_) @indent
  (#not-same-line? @indent @expr-start)
  (#set! "scope" "all")
)
(let_declaration
  .
  (_) @expr-start
  value: (_) @indent
  (#not-same-line? @indent @expr-start)
  (#set! "scope" "all")
)
(if_let_expression
  .
  (_) @expr-start
  value: (_) @indent
  (#not-same-line? @indent @expr-start)
  (#set! "scope" "all")
)
(static_item
  .
  (_) @expr-start
  value: (_) @indent
  (#not-same-line? @indent @expr-start)
  (#set! "scope" "all")
)

; Some field expressions where the left part is a multiline expression are not
; indented by cargo fmt.
; Because this multiline expression might be nested in an arbitrary number of
; field expressions, this can only be matched using a Regex.
(field_expression
  value: (_) @val
  "." @outdent
  (#match? @val "(\\A[^\\n\\r]+\\([\\t ]*(\\n|\\r).*)|(\\A[^\\n\\r]*\\{[\\t ]*(\\n|\\r))")
)
