; highlights.scm


; Literals

(integer) @number

(float) @float

(complex) @number

(string) @string
(string (escape_sequence) @string.escape)

(comment) @comment

(identifier) @variable

(formal_parameters (identifier) @parameter)
(formal_parameters (default_parameter (identifier) @parameter))

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
] @conditional

[
  "while"
  "repeat"
  "for"
] @repeat

[
  (true)
  (false)
] @boolean

"function" @keyword.function

(call function: (identifier) @function)
(default_argument name: (identifier) @parameter)


(namespace_get function: (identifier) @method)
(namespace_get_internal function: (identifier) @method)

(namespace_get namespace: (identifier) @namespace
 "::" @operator)
(namespace_get_internal namespace: (identifier) @namespace
 ":::" @operator)

; Error
(ERROR) @error
