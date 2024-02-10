; Indentation queries for helix
[
  (function_expression)
  (array_literal)
  (dictionary_literal)
  (selection_statement)
  (iteration_statement)
] @indent

; question - what about else, elif
[
  ")"
  "]"
  "}"
  (endif)
  (endforeach)
] @outdent
