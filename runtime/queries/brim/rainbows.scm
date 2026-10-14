[
  ; {/}
  (block)
  (struct_declaration)
  (enum_declaration)
  (impl_declaration)
  (match_statement)
  (struct_expression)
  (import_declaration)

  ; (/)
  (parameters)
  (arguments)
  (parenthesized_expression)
  (variant)
  (variant_pattern)

  ; [/]
  (type_parameters)
  (type_arguments)
  (generic_type)
  (list_expression)
  (map_expression)
  (index_expression)
] @rainbow.scope

[ "(" ")" "[" "]" "{" "}" ] @rainbow.bracket
