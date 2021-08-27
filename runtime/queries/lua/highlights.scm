;;; Highlighting for lua

;;; Builtins
(self) @variable.builtin

;; Keywords

(if_statement
[
  "if"
  "then"
  "end"
] @conditional)

[
  "else"
  "elseif"
  "then"
] @conditional

(for_statement
[
  "for"
  "do"
  "end"
] @repeat)

(for_in_statement
[
  "for"
  "do"
  "end"
] @repeat)

(while_statement
[
  "while"
  "do"
  "end"
] @repeat)

(repeat_statement
[
  "repeat"
  "until"
] @repeat)

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
] @keyword

"return" @keyword.return

;; Operators

[
 "not"
 "and"
 "or"
] @keyword.operator

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

;; Variables
(identifier) @variable

;; Constants
[
(false)
(true)
] @boolean
(nil) @constant.builtin
(spread) @constant ;; "..."
((identifier) @constant
 (#match? @constant "^[A-Z][A-Z_0-9]*$"))

;; Functions
(function [(function_name) (identifier)] @function)
(function ["function" "end"] @keyword.function)

(local_function (identifier) @function)
(local_function ["function" "end"] @keyword.function)

(variable_declaration
 (variable_declarator (identifier) @function) (function_definition))
(local_variable_declaration
 (variable_declarator (identifier) @function) (function_definition))

(function_definition ["function" "end"] @keyword.function)

(property_identifier) @property

(function_call
  [((identifier) @variable (method) @method)
   ((_) (method) @method)
   (identifier) @function
   (field_expression (property_identifier) @function)]
  . (arguments))

;; Parameters
(parameters
  (identifier) @parameter)

;; Nodes
(table ["{" "}"] @constructor)
(comment) @comment
(string) @string
(number) @number
(label_statement) @label
; A bit of a tricky one, this will only match field names
(field . (identifier) @field (_))
(shebang) @comment

;; Error
(ERROR) @error
