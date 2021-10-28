;;; Highlighting for lua

;;; Builtins
(self) @variable.builtin

;; Keywords

(if_statement
[
  "if"
  "then"
  "end"
] @keyword.control.conditional)

[
  "else"
  "elseif"
  "then"
] @keyword.control.conditional

(for_statement
[
  "for"
  "do"
  "end"
] @keyword.control.repeat)

(for_in_statement
[
  "for"
  "do"
  "end"
] @keyword.control.repeat)

(while_statement
[
  "while"
  "do"
  "end"
] @keyword.control.repeat)

(repeat_statement
[
  "repeat"
  "until"
] @keyword.control.repeat)

(do_statement
[
  "do"
  "end"
] @keyword)

[
 "in"
 "local"
 (break_statement)
 "goto"
 "return"
] @keyword

;; Operators

[
 "not"
 "and"
 "or"
] @operator

[
"="
"~="
"=="
"<="
">="
"<"
">"
"+"
"-"
"%"
"/"
"//"
"*"
"^"
"&"
"~"
"|"
">>"
"<<"
".."
"#"
 ] @operator

;; Punctuation
["," "." ":" ";"] @punctuation.delimiter

;; Brackets
[
 "("
 ")"
 "["
 "]"
 "{"
 "}"
] @punctuation.bracket

; ;; Constants
[
(false)
(true)
] @constant.builtin.boolean
(nil) @constant.builtin
(spread) @constant ;; "..."
((identifier) @constant
 (#match? @constant "^[A-Z][A-Z_0-9]*$"))

;; Parameters
(parameters
  (identifier) @variable.parameter)

; ;; Functions
(function [(function_name) (identifier)] @function)
(function ["function" "end"] @keyword.function)

(function
  (function_name
   (function_name_field
    (property_identifier) @function .)))

(local_function (identifier) @function)
(local_function ["function" "end"] @keyword.function)

(variable_declaration
 (variable_declarator (identifier) @function) (function_definition))
(local_variable_declaration
 (variable_declarator (identifier) @function) (function_definition))

(function_definition ["function" "end"] @keyword.function)

(function_call
  [
   ((identifier) @variable (method) @function.method)
   ((_) (method) @function.method)
   (identifier) @function
   (field_expression (property_identifier) @function)
  ]
  . (arguments))

;; Nodes
(table ["{" "}"] @constructor)
(comment) @comment
(string) @string
(number) @constant.numeric.integer
(label_statement) @label
; A bit of a tricky one, this will only match field names
(field . (identifier) @variable.other.member (_))
(shebang) @comment

;; Property
(property_identifier) @variable.other.member

;; Variable
(identifier) @variable

;; Error
(ERROR) @error
