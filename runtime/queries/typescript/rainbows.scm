; inherits: ecma

[
  (import_require_clause)
  (enum_body)
  (lookup_type)
  (parenthesized_type)
  (object_type)
  (type_parameters)
  (index_signature)
  (array_type)
  (tuple_type)
] @rainbow.scope

(type_arguments ["<" ">"] @rainbow.bracket) @rainbow.scope

[
  "{|" "|}"
] @rainbow.bracket
