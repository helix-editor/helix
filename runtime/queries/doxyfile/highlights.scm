(comment) @comment.line

(identifier) @variable

(boolean) @constant.builtin.boolean
(number) @constant.numeric.integer
[
  (unquoted_string)
  (quoted_string)
] @string

[
  "\\"
] @punctuation.delimiter

[
  "="
  "+="
] @operator
