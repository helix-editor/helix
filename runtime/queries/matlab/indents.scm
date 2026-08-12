[
  (arguments_statement)
  (if_statement)
  (for_statement)
  (while_statement)
  (switch_statement)
  (try_statement)
  (function_definition)
  (class_definition)
  (enumeration)
  (events)
  (methods)
  (properties)
] @indent

; switch arms add the second indent level (case keyword at switch level, body
; one deeper); they stay @indent.
[
  (case_clause)
  (otherwise_clause)
] @indent @extend

; if/try branch keywords align with the construct: the enclosing statement
; already indents their bodies, so the clauses are NOT @indent and the keyword
; outdents.
[
  "else"
  "elseif"
  "catch"
  "end"
] @outdent
