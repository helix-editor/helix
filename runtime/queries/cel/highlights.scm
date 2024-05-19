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

; Identifiers

(identifier) @variable.other.member

(select_expression
  operand: (identifier) @type)

(select_expression
  operand: (select_expression
    member: (identifier) @type))

; Function calls

(call_expression
  function: (identifier) @function)

(member_call_expression
  function: (identifier) @function)

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
] @constant.numeric.integer
(float_literal) @constant.numeric.float

[
  (true)
  (false)
] @constant.builtin.boolean

(null) @constant.builtin

(comment) @comment
