[
  (function_clause)
  (case)
  (clause)
  (anonymous_function)
  (if)
  (receive)
  (try)
  (block)
  (maybe)
  (list)
  (tuple)
  (map)
  (arguments)
  (bitstring)
] @indent

; `catch`/`after` are branch keywords inside `try`/`receive`; they align with
; the construct, not its body.
[
  "end"
  "catch"
  "after"
  "]"
  "}"
  ")"
  ">>"
] @outdent

; Handle partially typed code.
(ERROR "->" @indent @extend)
(ERROR "of" @indent @extend)
