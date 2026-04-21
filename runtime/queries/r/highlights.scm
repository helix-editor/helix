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
  (inf)
] @constant.numeric.float

[
  (true)
  (false)
] @constant.builtin.boolean

[
  (na)
  (null)
  (dots)
  (dot_dot_i)
] @constant.builtin

(string) @string
(string (string_content (escape_sequence) @constant.character.escape))

; Comments

(comment) @comment

; Operators

[
  "!" "!=" "$" "&" "&&" "*" "**" "+" "-" "->" "->>" "/" ":" ":::" ":::" ":=" "<"
  "<-" "<<-" "<=" "=" "==" ">" ">=" "?" "@" "^" "special" "|" "|>" "||" "~"
] @operator

(function_definition name: "\\" @operator)

; Punctuation

[
  "(" ")"
  "[" "]"
  "{" "}"
  "[[" "]]"
] @punctuation.bracket

(comma) @punctuation.delimiter

; Functions

(binary_operator
  lhs: (identifier) @function
  operator: "<-"
  rhs: (function_definition))

(binary_operator
  lhs: (identifier) @function
  operator: "="
  rhs: (function_definition))

; Calls

(call function: (identifier) @function)
(call function: (namespace_operator rhs: (identifier) @function))

; Parameters

(parameters (parameter name: (identifier) @variable.parameter))
(arguments (argument name: (identifier) @variable.parameter))

; Namespaces

(namespace_operator lhs: (identifier) @namespace)

; Keywords

[
  "in"
  (next)
  (break)
] @keyword

(return) @keyword.control.return

[
  "if"
  "else"
] @keyword.control.conditional

[
  "while"
  "repeat"
  "for"
] @keyword.control.repeat

"function" @keyword.function

