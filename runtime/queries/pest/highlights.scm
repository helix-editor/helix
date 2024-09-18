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
  "PUSH"
  "PEEK"
  "POP"
  "SOI"
  "EOI"
  "ANY"
] @keyword

