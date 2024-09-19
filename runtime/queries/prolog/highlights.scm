(comment) @comment @spell

(atom) @constant

((atom) @boolean
  (#eq? @boolean "true"))

((atom) @boolean
  (#eq? @boolean "false"))

(functional_notation
  function: (atom) @function.call)

(integer) @number

(float_number) @number.float

(directive_head) @operator

(operator_notation
  operator: _ @operator)

[
 (open)
 (open_ct)
 (close)
 (open_list)
 "|"
 (close_list)
 (open_curly)
 (close_curly)
] @punctuation.bracket

[
 (arg_list_separator)
 (comma)
 (end)
 (list_notation_separator)
] @punctuation.delimiter

(operator_notation
  operator: (semicolon) @punctuation.delimiter)

(double_quoted_list_notation) @string

(variable_term) @variable
