(string_literal) @string

(boolean_literal) @constant.builtin.boolean
(integer_literal) @constant.numeric.integer

(comment) @comment.line
(function_id) @function
(keyword_arg_key) @variable.other.member
(id_expression) @variable

[
  "if"
  "elif"
  "else"
  "endif"
] @keyword.control.conditional

[
  "foreach"
  "endforeach"
] @keyword.control.repeat

[
  "break"
  "continue"
] @keyword.control

[
  "not"
  "in"
  "and"
  "or"
] @keyword.operator

[
  "!"
  "+"
  "-"
  "*"
  "/"
  "%"
  "=="
  "!="
  ">"
  "<"
  ">="
  "<="
] @operator

[
  ":"
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
