; upstream: https://github.com/tree-sitter/tree-sitter-regex/blob/e1cfca3c79896ff79842f057ea13e529b66af636/queries/highlights.scm

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

(group_name) @label

(count_quantifier
  [
    (decimal_digits) @constant.numeric
    "," @punctuation.delimiter
  ])

(character_class
  [
    "^" @operator
    (class_range "-" @operator)
  ])

(class_character) @constant.character
