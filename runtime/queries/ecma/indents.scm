[
  (array)
  (object)
  (arguments)
  (formal_parameters)

  (statement_block)
  (switch_statement)
  (object_pattern)
  (class_body)
  (named_imports)

  (binary_expression)
  (return_statement)
  (template_substitution)
  (export_clause)

  (member_expression)
  (call_expression)
] @indent

[
  (switch_case)
  (switch_default)
] @indent

[
  "}"
  "]"
  ")"
] @outdent

; Single-statement bodies without braces (if/else/while/for/do). Indent the
; statement node so a newline typed after the header indents — capturing the
; body child alone only reindents existing lines, since the typing-direction
; walk reaches the statement, not its body child. The else-bearing if keeps the
; consequence-child form (scope all) so the `else` line itself isn't indented;
; a braceless else/do body is handled via its own node for the same reason.
(if_statement
  consequence: (_) @_body
  (#not-kind-eq? @_body "statement_block")
  !alternative) @indent
(if_statement
  consequence: (_) @indent
  (#not-kind-eq? @indent "statement_block")
  alternative: (_)
  (#set! "scope" "all"))
(else_clause
  (_) @_body
  (#not-kind-eq? @_body "statement_block")
  (#not-kind-eq? @_body "if_statement")) @indent
(while_statement
  body: (_) @_body
  (#not-kind-eq? @_body "statement_block")) @indent
(for_statement
  body: (_) @_body
  (#not-kind-eq? @_body "statement_block")) @indent
(for_in_statement
  body: (_) @_body
  (#not-kind-eq? @_body "statement_block")) @indent
(do_statement
  body: (_) @indent
  (#not-kind-eq? @indent "statement_block")
  (#set! "scope" "all"))

; Template-literal bodies are literal content (between interpolations).
(template_string) @opaque
