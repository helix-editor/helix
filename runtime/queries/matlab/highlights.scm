; Constants

(events (identifier) @constant)
(attribute (identifier) @constant)

"~" @constant.builtin

; Fields/Properties

(superclass "." (identifier) @variable.other.member)
(property_name "." (identifier) @variable.other.member)
(property name: (identifier) @variable.other.member)

; Types

(class_definition name: (identifier) @keyword.storage.type)
(attributes (identifier) @constant)
(enum . (identifier) @type.enum.variant)

; Functions

(function_definition
  "function" @keyword.function
  name: (identifier) @function
  [ "end" "endfunction" ]? @keyword.function)

(function_signature name: (identifier) @function)
(function_call name: (identifier) @function)
(handle_operator (identifier) @function)
(validation_functions (identifier) @function)
(command (command_name) @function.macro)
(command_argument) @string
(return_statement) @keyword.control.return

; Assignments

(assignment left: (_) @variable)
(multioutput_variable (_) @variable)

; Parameters

(function_arguments (identifier) @variable.parameter)

; Conditionals

(if_statement [ "if" "end" ] @keyword.control.conditional)
(elseif_clause "elseif" @keyword.control.conditional)
(else_clause "else" @keyword.control.conditional)
(switch_statement [ "switch" "end" ] @keyword.control.conditional)
(case_clause "case" @keyword.control.conditional)
(otherwise_clause "otherwise" @keyword.control.conditional)
(break_statement) @keyword.control.conditional

; Repeats

(for_statement [ "for" "parfor" "end" ] @keyword.control.repeat)
(while_statement [ "while" "end" ] @keyword.control.repeat)
(continue_statement) @keyword.control.repeat

; Exceptions

(try_statement [ "try" "end" ] @keyword.control.exception)
(catch_clause "catch" @keyword.control.exception)

; Punctuation

[ ";" "," "." ] @punctuation.delimiter
[ "(" ")" "[" "]" "{" "}" ] @punctuation.bracket

; Literals

(escape_sequence) @constant.character.escape
(formatting_sequence) @constant.character.escape
(string) @string
(number) @constant.numeric.float
(unary_operator ["+" "-"] @constant.numeric.float)
(boolean) @constant.builtin.boolean

; Comments

[ (comment) (line_continuation) ] @comment.line

; Operators

[
  "+"
  ".+"
  "-"
  ".*"
  "*"
  ".*"
  "/"
  "./"
  "\\"
  ".\\"
  "^"
  ".^"
  "'"
  ".'"
  "|"
  "&"
  "?"
  "@"
  "<"
  "<="
  ">"
  ">="
  "=="
  "~="
  "="
  "&&"
  "||"
  ":"
] @operator

; Keywords

"classdef" @keyword.storage.type
[
  "arguments"
  "end"
  "enumeration"
  "events"
  "global"
  "methods"
  "persistent"
  "properties"
] @keyword
