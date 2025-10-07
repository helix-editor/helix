[
  (indented_string_expression)
  (string_expression)

  ; these are all direct parents of (binding_set)
  (attrset_expression)
  (let_attrset_expression)
  (rec_attrset_expression)
  (let_expression)

  (list_expression)
  (parenthesized_expression)
] @indent


(if_expression [ "if" "then" "else" ] @align)

[
  "}"
  "]"
  ")"
] @outdent
