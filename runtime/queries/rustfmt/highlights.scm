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

(ERROR) @error
