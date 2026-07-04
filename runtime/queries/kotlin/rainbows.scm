[
  (catch_block)
  (class_body)
  (collection_literal)
  (delegation_specifier)
  (do_while_statement)
  (enum_class_body)
  (file_annotation)
  (for_statement)
  (function_type_parameters)
  (function_value_parameters)
  (getter)
  (if_expression)
  (indexing_suffix)
  (lambda_literal)
  (multi_variable_declaration)
  (parenthesized_expression)
  (parenthesized_type)
  (parenthesized_user_type)
  (setter)
  (value_arguments)
  (when_expression)
  (when_subject)
  (while_statement)
] @rainbow.scope

(type_arguments ["<" ">"] @rainbow.bracket) @rainbow.scope
(type_parameters ["<" ">"] @rainbow.bracket) @rainbow.scope

[
  "(" ")"
  "{" "}"
  "[" "]"
] @rainbow.bracket
