"if" @keyword.control.conditional
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
(fieldname) @string.special
[
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket
