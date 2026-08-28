; Bracket pairs for cursor navigation

[
  (for_statement)
  (for_in_statement)
  (for_of_statement)
  (catch_clause)
  (formal_parameters)
  (parenthesized_expression)
  (arguments)
  (parenthesized_type)
  (array_pattern)
  (subscript_expression)
  (computed_property_name)
  (array)
  (array_type)
  (tuple_type)
  (export_clause)
  (named_imports)
  (statement_block)
  (switch_body)
  (component_body)
  (class_body)
  (object_pattern)
  (server_block)
  (object)
  (jsx_expression)
  (object_type)
] @rainbow.scope

(jsx_opening_element ["<" ">"] @rainbow.bracket) @rainbow.scope
(jsx_closing_element ["</" ">"] @rainbow.bracket) @rainbow.scope
(jsx_self_closing_element ["<" "/>"] @rainbow.bracket) @rainbow.scope

[
  "(" ")"
  "[" "]"
  "{" "}"
] @rainbow.bracket
