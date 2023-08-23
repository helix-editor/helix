; Operators in command and conditional HLL expressions
(hll_comma_expression
  "," @operator)

(hll_conditional_expression
  [
   "?"
   ":"
] @operator)


; Keywords, punctuation and operators
[
  "enum"
  "struct"
  "union"
] @keyword.storage.type

"sizeof" @keyword.operator

[
  "const"
  "volatile"
] @keyword.storage.modifier

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
  "-="
  "+="
  "*="
  "/="
  "%="
  "|="
  "&="
  "^="
  ">>="
  "<<="
  "--"
  "++"
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
] @punctuation.delimiter


; Strings and others literal types
(access_class) @constant.builtin

[
  (address)
  (bitmask)
  (file_handle)
  (integer)
  (hll_number_literal)
] @constant.numeric.integer

[
  (float)
  (frequency)
  (percentage)
  (time)
] @constant.numeric.float

[
  (string)
  (hll_string_literal)
] @string

(hll_escape_sequence) @constant.character.escape

(path) @string.special.path
(symbol) @string.special.symbol

[
  (character)
  (hll_char_literal)
] @constant.character


; Types in HLL expressions
[
  (hll_type_identifier)
  (hll_type_descriptor)
] @type

(hll_type_qualifier) @keyword.storage.modifier

(hll_primitive_type) @type.builtin


; HLL call expressions
(hll_call_expression
  function: (hll_field_expression
    field: (hll_field_identifier) @function))

(hll_call_expression
  function: (identifier) @function)


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
(macro) @variable.builtin
(trace32_hll_variable) @variable.builtin

(
  (command_expression
    command: (identifier) @keyword
    arguments: (argument_list . (identifier) @label))
  (#match? @keyword "^[gG][oO][tT][oO]$")
)
(labeled_expression
  label: (identifier) @label)

(option_expression
  (identifier) @constant.builtin)

(format_expression
  (identifier) @constant.builtin)

(
  (argument_list (identifier) @constant.builtin)
  (#match? @constant.builtin "^[%/][a-zA-Z][a-zA-Z0-9.]*$")
)
(argument_list
  (identifier) @constant.builtin)


; Commands
(command_expression command: (identifier) @keyword)
(macro_definition command: (identifier) @keyword)

(call_expression
  function: (identifier) @function.builtin)


; Control flow
(if_block
  command: (identifier) @keyword.control.conditional.if)
(else_block
  command: (identifier) @keyword.control.control.else)

(while_block
  command: (identifier) @keyword.control.repeat.while)
(repeat_block
  command: (identifier) @keyword.control.loop)


; HLL variables
(identifier) @variable
(hll_field_identifier) @variable.other.member


(comment) @comment
