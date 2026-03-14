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
  (#set! "scope" "all")
)

(control_transfer_statement
  .
  (_) @expr-start
  (_) @indent
  (#not-same-line? @indent @expr-start)
  (#set! "scope" "all")
)

(if_statement
  (if_statement) @outdent
)

(switch_entry
  .
  _ @indent
  (#set! "scope" "tail")
)

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
