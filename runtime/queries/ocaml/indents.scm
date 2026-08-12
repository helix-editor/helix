[
  (let_binding)
  (type_binding)
  (structure)
  (signature)
  (record_declaration)
  (function_expression)
  (fun_expression)
  (match_case)
  (then_clause)
  (else_clause)
] @indent

; `else if` chains nest else_clause -> if_expression -> else_clause, which would
; double-indent the trailing branch; `try`/`with` and explicit module `end`
; alignment are likewise left to the writer — see tests/indent/ocaml.ml.
[
  "}"
  ")"
  "]"
  "end"
] @outdent
