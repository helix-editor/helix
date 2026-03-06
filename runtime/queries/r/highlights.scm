; highlights.scm

(identifier) @variable

; Literals

[
  (integer)
  (complex)
] @constant.numeric.integer

[
  (float)
  (nan)
] @constant.numeric.float

[
  (true)
  (false)
] @constant.builtin.boolean

[
  (na)
  (null)
] @constant.builtin

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
  "if"
  "else"
  "switch"
] @keyword.control.conditional

[
  "while"
  "repeat"
  "for"
] @keyword.control.repeat

"function" @keyword.function

(call function: (identifier) @function)
(default_argument name: (identifier) @variable.parameter)


(namespace_get namespace: (identifier) @namespace
 "::" @operator)
(namespace_get_internal namespace: (identifier) @namespace
 ":::" @operator)

(namespace_get function: (identifier) @function.method)
(namespace_get_internal function: (identifier) @function.method)
