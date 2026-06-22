[
  (anonymous_function)
  (do_block)
  (stab_clause)
  (map)
  (list)
  (tuple)
] @indent

; The else/rescue/catch/after clause blocks nest *inside* the do_block, which
; already provides the body's one level of indent; the keyword itself outdents
; back to the construct's level.
[
  "end"
  "else"
  "rescue"
  "catch"
  "after"
  "]"
  "}"
  ")"
] @outdent
