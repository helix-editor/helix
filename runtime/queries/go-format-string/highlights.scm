(escaped_percent_sign) @constant.character.escape

"." @punctuation.delimiter
"%" @punctuation.special

[
  "["
  "]"
] @punctuation.bracket

(explicit_argument_index) @constant.numeric

(flag) @constant.builtin

(width) @constant.numeric.integer
(precision) @constant.numeric.float
(asterisk) @string.special.symbol

(verb) @type

(text) @string
