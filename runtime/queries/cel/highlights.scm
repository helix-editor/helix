; Operators

[
  "-"
  "!"
  "*"
  "/"
  "&&"
  "%"
  "+"
  "<"
  "<="
  "=="
  ">"
  ">="
  "||"
] @operator

; Keywords

[
"in"
] @keyword

; Function calls

(call_expression
  function: (identifier) @function)

(member_call_expression
  function: (identifier) @function)

; Identifiers

(select_expression
  operand: (identifier) @type)

(select_expression
  operand: (select_expression
    member: (identifier) @type))

(identifier) @property

; Literals

[
  (double_quote_string_literal)
  (single_quoted_string_literal)
  (triple_double_quote_string_literal)
  (triple_single_quoted_string_literal)
] @string

[
  (int_literal)
  (uint_literal)
  (float_literal)
] @number

[
  (true)
  (false)
  (null)
] @constant.builtin

(comment) @comment
