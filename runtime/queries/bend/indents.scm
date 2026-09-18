[
  (block)
  (match_cases)
  (constructors)
  (do_body)
  (law_body)
] @indent

; Keep indentation active while editing an incomplete suite.
[
  (type_declaration)
  (law_declaration)
  (match_expression)
  (case_clause)
  (function_definition)
  (do_expression)
] @extend

; Delimited expressions align naturally after their opening delimiter and
; return to the surrounding indentation at the closing delimiter.
[
  (parameters)
  (arguments)
  (list_expression)
  (tuple_expression)
  (parenthesized_expression)
  (constructor_expression)
  (type_application)
  (type_parameters)
] @indent

[
  ")"
  "]"
  "}"
] @outdent

(comment) @indent.always
