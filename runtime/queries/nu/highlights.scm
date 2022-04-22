(string) @string
(type) @type
(value_path) @variable
(comment) @comment

(number_literal) @constant.numeric
(range from: (number_literal) @constant.numeric)
(range to: (number_literal) @constant.numeric)

(command cmd_name: (identifier) @function)
(function_definition func_name: (identifier) @function)

[
    (variable_declaration name: (identifier))
    (parameter (identifier))
    (flag (flag_name))
    (flag (flag_shorthand_name))
    (record_entry entry_name: (identifier))
    (block_args block_param: (identifier))
] @variable.other.member
; (parameter (identifier) @variable.parameter) ; -- alternative highlighting group?

(cmd_invocation) @embedded


((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]*$"))

[
 "if"
 "else"
 "let"
 "def"
 "export"
] @keyword

[
  ; "/" Not making / an operator may lead to better highlighting?
  "$"
  "|"
  "+"
  "-"
  "*"
  "="
  "!="
  "&&"
  "||"
  "=="
  ">"
] @operator

["."
 "," 
 ";" 
] @punctuation.delimiter
