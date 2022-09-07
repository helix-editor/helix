["if" "then" "else"] @conditional
[
  (local)
  "function"
] @keyword
(comment) @comment

(string) @string
(number) @number
[
  (true)
  (false)
] @boolean

(binaryop) @operator
(unaryop) @operator

(param identifier: (id) @variable.parameter)
(bind function: (id) @function)
(fieldname) @string.special
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
(id) @variable
