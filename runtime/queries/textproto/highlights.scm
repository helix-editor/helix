(string) @string

(field_name) @variable.other.member

(comment) @comment

(number) @constant.numeric
; covers e.g. booleans and "inf"
(scalar_value (identifier)) @constant
; Covers "-inf"
(scalar_value (signed_identifier)) @constant.numeric

[
  (open_squiggly)
  (close_squiggly)
  (open_square)
  (close_square)
  (open_arrow)
  (close_arrow)
] @punctuation.bracket

"," @punctuation.delimiter
