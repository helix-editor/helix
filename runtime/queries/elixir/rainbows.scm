[
  (block)
  (interpolation)
  (list)
  (tuple)
  (bitstring)
  (map)
  ; short-hand function captures like &(&1 + &2)
  (unary_operator
    operator: "&")
  (arguments "(" ")")
  (access_call)
  (sigil)
] @rainbow.scope

[
  "(" ")"
  "%"
  "{" "}"
  "[" "]"
  "<<" ">>"
  "#{"
  "|"
] @rainbow.bracket
