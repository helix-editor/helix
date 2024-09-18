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

  (token_tree)
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
  "let" @expr-start
  value: (_) @indent
  alternative: (_)? @indent
  (#not-same-line? @indent @expr-start)
  (#set! "scope" "all")
)
(let_condition
  .
  (_) @expr-start
  value: (_) @indent
  (#not-same-line? @indent @expr-start)
  (#set! "scope" "all")
)
(if_expression
  .
  (_) @expr-start
  condition: (_) @indent
  (#not-same-line? @indent @expr-start)
  (#set! "scope" "all")
)
(field_pattern
  .
  (_) @expr-start
  pattern: (_) @indent
  (#not-same-line? @indent @expr-start)
  (#set! "scope" "all")
)
; Indent type aliases that span multiple lines, similar to
; regular assignment expressions
(type_item
  .
  (_) @expr-start
  type: (_) @indent
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
  ; Check whether the first line ends with `(`, `{` or `[` (up to whitespace).
  (#match? @val "(\\A[^\\n\\r]+(\\(|\\{|\\[)[\\t ]*(\\n|\\r))")
)
; Same as above, but with an additional `call_expression`. This is required since otherwise
; the arguments of the function call won't be outdented.
(call_expression
  function: (field_expression
    value: (_) @val
    "." @outdent
    (#match? @val "(\\A[^\\n\\r]+(\\(|\\{|\\[)[\\t ]*(\\n|\\r))")
  )
  arguments: (_) @outdent
)


; Indent if guards in patterns.
; Since the tree-sitter grammar doesn't create a node for the if expression,
; it's not possible to do this correctly in all cases. Indenting the tail of the
; whole pattern whenever it contains an `if` only fails if the `if` appears after
; the second line of the pattern (which should only rarely be the case)
(match_pattern
  .
  (_) @expr-start
  "if" @pattern-guard
  (#not-same-line? @expr-start @pattern-guard)
) @indent

(for_expression
  "in" @in
  .
  (_) @indent
  (#not-same-line? @in @indent)
  (#set! "scope" "all")
)  
