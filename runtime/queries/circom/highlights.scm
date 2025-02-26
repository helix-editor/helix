; identifiers
; -----------
(identifier) @variable

; Pragma
; -----------
(pragma_directive) @keyword.directive

; Include
; -----------
(include_directive) @keyword.directive

; Literals
; --------
(string) @string
(int_literal) @constant.numeric.integer
(comment) @comment

; Definitions
; -----------
(function_definition
  name:  (identifier) @keyword.function)

(template_definition
  name:  (identifier) @keyword.function)

; Use constructor coloring for special functions
(main_component_definition) @constructor

; Invocations
(call_expression . (identifier) @function)

; Function parameters
(parameter name: (identifier) @variable.parameter)

; Members
(member_expression property: (property_identifier) @variable.other.member)

; Tokens
; -------

; Keywords
[
 "signal"
 "var"
 "component"
] @keyword.storage.type

[  "include" ] @keyword.control.import

[
 "public"
 "input"
 "output"
 ] @keyword.storage.modifier

[
 "for"
 "while"
] @keyword.control.repeat

[
 "if"
 "else"
] @keyword.control.conditional

[
 "return"
] @keyword.control.return

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
  ";"
] @punctuation.delimiter

; Operators
; https://docs.circom.io/circom-language/basic-operators
[
  "="
  "?"
  "&&"
  "||"
  "!"
  "<" 
  ">" 
  "<=" 
  ">=" 
  "==" 
  "!=" 
  "+"
  "-"
  "*"
  "**"
  "/"
  "\\"
  "%"
  "+="
  "-="
  "*="
  "**="
  "/="
  "\\="
  "%="
  "++"
  "--"
  "&"
  "|"
  "~"
  "^"
  ">>"
  "<<"
  "&="
  "|="
  ; "\~=" ; bug, uncomment and circom will not highlight
  "^="
  ">>="
  "<<="
] @operator

[
  "<=="
  "==>"
  "<--"
  "-->"
  "==="
] @operator
