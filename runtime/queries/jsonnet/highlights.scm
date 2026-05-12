["if" "then" "else"] @keyword.control.conditional
[
  (local)
  "function"
] @keyword
(comment) @comment

(string) @string
(number) @constant.numeric
[
  (true)
  (false)
] @constant.builtin.boolean

(binaryop) @operator
(unaryop) @operator

(id) @variable
(param identifier: (id) @variable.parameter)
(bind function: (id) @function)
(fieldname (id) @variable.other.member)
[
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket
"for" @keyword.control.repeat
"in" @keyword.operator
[(self) (dollar)] @variable.builtin
"assert" @keyword
(null) @constant.builtin
[
  ":"
  "::"
  ";"
  "="
] @punctuation.delimiter
