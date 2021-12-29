[
  "("
  ")"
  "(?"
  "(?:"
  "(?<"
  ">"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  "*"
  "+"
  "|"
  "="
  "<="
  "!"
  "<!"
  "?"
] @operator

[
  (identity_escape)
  (control_letter_escape)
  (character_class_escape)
  (control_escape)
  (start_assertion)
  (end_assertion)
  (boundary_assertion)
  (non_boundary_assertion)
] @constant.character.escape

(group_name) @property

(count_quantifier
  [
    (decimal_digits) @constant.numeric
    "," @punctuation.delimiter
  ])

(character_class
  [
    "^" @operator
    (class_range "-" @operator)
    (class_character) @constant.character
  ])
