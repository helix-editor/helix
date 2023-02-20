(ERROR) @error

(line_comment) @comment

; keywords and symbols

(keyword) @keyword
(symbol) @tag

; literals

(bool_literal) @constant.builtin.boolean
(num_literal) @constant.numeric

; strings
(string_interpolation
  (string_interpolation_start) @punctuation.special
  (string_interpolation_end) @punctuation.special)

(escape_sequence) @constant.character.escape

(string
  [
    (unescaped_single_quote_string_fragment)
    (unescaped_double_quote_string_fragment)
    (unescaped_backtick_string_fragment)
    "\""
    "'"
    "`"
  ]) @string

; operators and general punctuation

(unary_expression
  operator: _ @operator)

(binary_expression
  operator: _ @operator)

(ternary_expression
  operator: _ @operator)

[
  ":"
  "."
  ","
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket
[
  ":"
  "."
  ","
] @punctuation.delimiter

; Rest (general identifiers that are not yet catched)

(index) @variable
(ident) @variable
