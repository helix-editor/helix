[
  (protocol_body)
  (class_body)
  (enum_class_body)
  (function_declaration)
  (init_declaration)
  (deinit_declaration)
  (computed_property)
  (subscript_declaration)
  (computed_getter)
  (computed_setter)
  (for_statement)
  (while_statement)
  (repeat_while_statement)
  (do_statement)
  (if_statement)
  (switch_statement)
  (guard_statement)
  (type_parameters)
  (tuple_type)
  (array_type)
  (dictionary_type)
  (call_expression)
  (tuple_expression)
  (array_literal)
  (dictionary_literal)
  (lambda_literal)
  (willset_didset_block)
  (willset_clause)
  (didset_clause)
  (value_arguments)
] @indent

[
  "}"
  "]"
  ")"
  ">"
] @outdent

(assignment
  .
  (_) @expr-start
  (_) @indent
  (#not-same-line? @indent @expr-start)

)

(control_transfer_statement
  .
  (_) @expr-start
  (_) @indent
  (#not-same-line? @indent @expr-start)

)

(if_statement
  (if_statement) @outdent
)

; switch_entry wraps both the case/default label and the body statements. The
; default tail scope indents the lines after the label (the body) without
; indenting the label itself, and because switch_entry is an ancestor of the
; cursor whether reindenting a body line or typing a newline after the label,
; it resolves correctly in both indent directions (capturing the inner
; (statements) node only works when reindenting; capturing the label only works
; when typing — the wrapper handles both).
(switch_entry) @indent

(init_declaration
  (parameter) @indent
)

(modifiers
  (attribute) @indent
)

(type_parameters
  ">" @outdent
)

(tuple_expression
  ")" @outdent
)

(tuple_type
  ")" @outdent
)

(modifiers
  (attribute
    ")" @outdent
  )
)

(ERROR
  [
    "<"
    "{"
    "("
    "["
  ]
) @indent

[
  (multi_line_string_literal)
  (raw_string_literal)
] @opaque
