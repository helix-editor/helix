(line_comment) @comment
(block_comment) @comment

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

((identifier) @variable)
((builtin) @type.builtin)
((const) @constant)

[
  (string)
  (character)
] @string

[
  "_"
  "@"
  "$"
]@keyword.storage.modifier

[
  "~"
  "|"
  "="
  "+"
  "*"
  "&"
  "^"
  "!"
  "?"
  ".."
] @operator

[
  "ANY"
  "DROP"
  "EOI"
  "NEWLINE"
  "PEEK"
  "PEEK_ALL"
  "POP"
  "POP_ALL"
  "PUSH"
  "SOI"
] @keyword
