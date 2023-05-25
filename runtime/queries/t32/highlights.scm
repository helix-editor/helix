[
  "="
  "^^"
  "||"
  "&&"
  "+"
  "-"
  "*"
  "/"
  "%"
  "|"
  "^"
  "=="
  "!="
  ">"
  ">="
  "<="
  "<"
  "<<"
  ">>"
  ".."
  "--"
  "++"
  "+"
  "-"
  "~"
  "!"
  "&"
  "->"
  "*"
] @operator

[
 "("
 ")"
 "{"
 "}"
 "["
 "]"
] @punctuation.bracket

[
  ","
  "."
  ";"
] @punctuation.delimiter

; Constants
[
  (access_class)
  (address)
  (bitmask)
  (file_handle)
  (frequency)
  (time)
] @constant.builtin

[
  (float)
  (percentage)
] @constant.numeric.float

(integer) @constant.numeric.integer

(character) @constant.character

; Strings
(string) @string

(path) @string.special.path

(symbol) @string.special.symbol

; Returns
(
  (command_expression
    command: (identifier) @keyword.return)
  (#match? @keyword.return "^[eE][nN][dD]([dD][oO])?$")
)
(
  (command_expression
    command: (identifier) @keyword.return)
  (#match? @keyword.return "^[rR][eE][tT][uU][rR][nN]$")
)

; Subroutine calls
(subroutine_call_expression
  command: (identifier) @keyword
  subroutine: (identifier) @function)

; Subroutine blocks
(subroutine_block
  command: (identifier) @keyword
  subroutine: (identifier) @function)

(labeled_expression
  label: (identifier) @function
  (block))

; Parameter declarations
(parameter_declaration
  command: (identifier) @keyword
  (identifier)? @constant.builtin
  macro: (macro) @variable.parameter)

; Variables, constants and labels
(macro) @variable
(internal_c_variable) @variable

(
  (command_expression
    command: (identifier) @keyword
    arguments: (argument_list . (identifier) @label))
  (#match? @keyword "^[gG][oO][tT][oO]$")
)
(labeled_expression
  label: (identifier) @label)

(
 (argument_list (identifier) @constant.builtin)
 (#match? @constant.builtin "^[%/][a-zA-Z][a-zA-Z0-9.]*$")
)
(argument_list
  (identifier) @constant)

; Commands
(command_expression command: (identifier) @keyword)
(macro_definition command: (identifier) @keyword)

; Control flow
(if_block
  command: (identifier) @keyword.control.conditional.if)
(else_block
  command: (identifier) @keyword.control.control.else)

(while_block
  command: (identifier) @keyword.control.repeat.while)
(repeat_block
  command: (identifier) @keyword.control.loop)

(call_expression
  function: (identifier) @function)

(type_identifier) @type
(comment) @comment
