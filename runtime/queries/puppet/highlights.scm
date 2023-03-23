(function) @function
(variable) @variable
[
  (identifier)
] @identifier

[
 "class"
 "define"
 "plan"
 "node"
 "type"
] @keyword

[
  "contain"
  "include"
  "inherits"
  "require"
] @keyword.control.import

[
 "case"
 "else"
 "elsif"
 "if"
 "unless"
] @keyword.control.conditional

(string) @string
(regex) @string.regex
(integer) @constant.numeric.integer
(float) @constant.numeric.float
(comment) @comment
[(true) (false)] @constant.builtin.boolean

(unprotected_string) @constant

[
 (chaining_arrow)
 (operator)
 "and"
 "or"
] @operator


(interpolation
  "${" @punctuation.special
  "}" @punctuation.special) @none

[
 "("
 ")"
 "["
 "]"
 "{"
 "}"
] @punctuation.bracket

[
 ","
 ":"
 "::"
] @punctuation.delimiter

[(type) (type_identifier)] @type

(escape_sequence) @escape

