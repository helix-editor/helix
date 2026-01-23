; Definition keywords
[
  (start_definition)
  (end_definition)
] @keyword

; Control flow - highlighted as keywords for prominence
(control_flow) @keyword.control

; I/O operations
(io) @function.builtin

; Operators - arithmetic, logic, stack manipulation
(operator) @operator

; Core builtins - defining words, memory, etc.
(core) @type

; Numbers - all subtypes
(character_literal) @constant.character
(hex_number) @constant.numeric
(binary_number) @constant.numeric
(octal_number) @constant.numeric
(float_number) @constant.numeric
(double_cell_number) @constant.numeric
(decimal_number) @constant.numeric

; Strings
(string) @string

; Comments - different types
(line_comment) @comment.line
(block_comment) @comment.block
(stack_effect) @comment.block.documentation

; User-defined words
(word) @function
