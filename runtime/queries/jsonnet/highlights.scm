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

; Binary operators are now an `operator:` field whose node is the operator
; category (additive/comparison/and/bitor/…); capture it directly.
(binary operator: _ @operator)
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
