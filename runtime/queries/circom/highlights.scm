; identifiers
; -----------
(identifier) @variable

; Pragma
; -----------
(pragma_directive) @tag

; Include
; -----------
(include_directive) @include

; Literals
; --------

(string) @string
(int_literal) @number
(comment) @comment

; Definitions
; -----------

(function_definition
  name:  (identifier) @function)

(template_definition
  name:  (identifier) @function)

; Use contructor coloring for special functions
(main_component_definition) @constructor

; Invocations

(call_expression . (identifier) @function)

; Function parameters
(parameter name: (identifier) @variable.parameter)


; Members
(member_expression property: (property_identifier) @property)


; Tokens
; -------

; Keywords

[
 "public"
 "signal"
 "var"
 "include"
 "input"
 "output"
 "public"
 "component"
] @keyword

[
 "for"
 "while"
] @repeat

[
 "if"
 "else"
] @conditional

[
 "return"
] @keyword.return

[
  "function"
  "template"
] @keyword.function


; Punctuation

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket


[
  "."
  ","
] @punctuation.delimiter


; Operators

[
  "&&"
  "||"
  ">>"
  "<<"
  "&"
  "^"
  "|"
  "+"
  "-"
  "*"
  "/"
  "%"
  "**"
  "<"
  "<="
  "=="
  "!="
  ">="
  ">"
  "!"
  "~"
  "-"
  "+"
  "++"
  "--"
] @operator

[
  "<=="
  "==>"
  "<--"
  "-->"
  "==="
] @assignment