[
  (anchor_begin)
  (anchor_end)
] @punctuation.delimiter

(pattern
  (character
    "." @variable.builtin))

[
  "["
  "]"
  "("
  ")"
] @punctuation.bracket

[
  (zero_or_more)
  (shortest_zero_or_more)
  (one_or_more)
  (zero_or_one)
] @operator

(range
  from: (character) @constant
  "-" @operator
  to: (character) @constant)

(set
  (character) @constant)

(negated_set
  (character) @constant)

(class) @constant.character.escape

(class
  "%" @string.regexp
  (escape_char) @string.regexp)

(negated_set
  "^" @operator)

(balanced_match
  (character) @variable.parameter)
