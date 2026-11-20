[
  (body)
  (body_expression)
  (argument_list)
  (sequence_expression)
  (multiplicity_range)
] @indent

[
  "}"
  ")"
  "]"
] @outdent

; Typing an opening brace without a closing one produces an ERROR node.
(ERROR "{") @indent @extend
(ERROR "(") @indent
