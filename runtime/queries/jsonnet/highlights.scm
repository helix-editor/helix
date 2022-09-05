"if" @conditional
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

(id) @variable
(param identifier: (id) @variable.parameter)
(bind function: (id) @function)
(fieldname) @string.special
[
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket
