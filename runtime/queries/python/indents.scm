[
  (list)
  (tuple)
  (dictionary)
  (set)

  (if_statement)
  (for_statement)
  (while_statement)
  (with_statement)
  (try_statement)
  (import_from_statement)

  (parenthesized_expression)
  (generator_expression)
  (list_comprehension)
  (set_comprehension)
  (dictionary_comprehension)

  (tuple_pattern)
  (list_pattern)
  (argument_list)
  (parameters)
  (binary_operator)

  (function_definition)
  (class_definition)
] @indent

[
  (if_statement)
  (for_statement)
  (while_statement)
  (with_statement)
  (try_statement)

  (function_definition)
  (class_definition)
] @extend-indented

[
  (return_statement)
  (break_statement)
  (continue_statement)
  (raise_statement)
  (pass_statement)
] @stop-extend

[
  ")"
  "]"
  "}"
] @outdent
(elif_clause
  "elif" @outdent)
(else_clause
  "else" @outdent)

