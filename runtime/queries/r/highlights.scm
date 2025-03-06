; highlights.scm

(identifier) @variable

; Literals

(integer) @constant.numeric.integer

(float) @constant.numeric.float

(complex) @constant.numeric.integer

(string) @string
(string (escape_sequence) @constant.character.escape)

(comment) @comment

(formal_parameters (identifier) @variable.parameter)
(formal_parameters (default_parameter (identifier) @variable.parameter))

; Operators
[
 "="
 "<-"
 "<<-"
 "->>"
 "->"
] @operator

(unary operator: [
  "-"
  "+"
  "!"
  "~"
] @operator)

(binary operator: [
  "-"
  "+"
  "*"
  "/"
  "^"
  "<"
  ">"
  "<="
  ">="
  "=="
  "!="
  "||"
  "|"
  "&&"
  "&"
  ":"
  "~"
] @operator)

[
  "|>"
  (special)
] @operator

(lambda_function "\\" @operator)

[
 "("
 ")"
 "["
 "]"
 "{"
 "}"
] @punctuation.bracket

(dollar "$" @operator)

(subset2
 [
  "[["
  "]]"
 ] @punctuation.bracket)

[
 "in"
 (dots)
 (break)
 (next)
 (inf)
] @keyword

[
  (nan)
  (na)
  (null)
] @type.builtin

[
  "if"
  "else"
  "switch"
] @keyword.control.conditional

[
  "while"
  "repeat"
  "for"
] @keyword.control.repeat

[
  (true)
  (false)
] @constant.builtin.boolean

"function" @keyword.function

(call function: (identifier) @function)
(default_argument name: (identifier) @variable.parameter)


(namespace_get namespace: (identifier) @namespace
 "::" @operator)
(namespace_get_internal namespace: (identifier) @namespace
 ":::" @operator)

(namespace_get function: (identifier) @function.method)
(namespace_get_internal function: (identifier) @function.method)
