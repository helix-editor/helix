[
  (string_type)
  (preamble_type)
  (entry_type)
] @keyword

[
  (junk)
  (comment)
] @comment

[
  "="
  "#"
] @operator

(command) @function.builtin

(number) @constant.numeric

(field
  name: (identifier) @variable.builtin)

(token
  (identifier) @variable.parameter)

[
  (brace_word)
  (quote_word)
] @string

[
  (key_brace)
  (key_paren)
] @attribute

(string
  name: (identifier) @constant)

[
  "{"
  "}"
  "("
  ")"
] @punctuation.bracket

"," @punctuation.delimiter
