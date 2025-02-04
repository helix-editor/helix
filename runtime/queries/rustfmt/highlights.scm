; (format_string) @string

(escaped) @constant.character.escape

[
  "#"
  (type)
] @special

[
  (sign)
  (fill)
  (align)
  (width)
] @operator

(number) @constant.numeric

(colon) @punctuation

(identifier) @variable

((identifier) @constant
 (#match? @constant "^[A-Z_]+$"))

[
  "{"
  "}"
] @punctuation.special
