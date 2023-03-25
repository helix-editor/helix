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
] @include

[
 "case"
 "else"
 "elsif"
 "if"
 "unless"
] @conditional

(string) @string
(regex) @regex
(integer) @number
(float) @float
(regex) @string.regex
(comment) @comment
[(true) (false)] @boolean

(unprotected_string) @text

[
 (chaining_arrow)
 (operator)
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
 ]@punctuation.bracket

[
 ","
 ":"
 "::"
] @punctuation.delimiter

[(type) (type_identifier)] @type

(escape_sequence) @escape

