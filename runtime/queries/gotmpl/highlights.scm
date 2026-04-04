; Identifiers

[
  (field)
  (field_identifier)
] @variable.other.member

(variable) @variable

; Function calls

(function_call
  function: (identifier) @function)

(method_call
  method: (selector_expression
    field: (field_identifier) @function))

; Operators

"|" @operator
"=" @operator
":=" @operator

; Builtin functions

((identifier) @function.builtin
 (#match? @function.builtin "^(and|call|html|index|slice|js|len|not|or|print|printf|println|urlquery|eq|ne|lt|ge|gt|ge)$"))

; Delimiters

"." @punctuation.delimiter
"," @punctuation.delimiter

"{{" @punctuation.bracket
"}}" @punctuation.bracket
"{{-" @punctuation.bracket
"-}}" @punctuation.bracket
")" @punctuation.bracket
"(" @punctuation.bracket

; Actions

(if_action
  [
    "if"
    "else"
    "end"
  ] @keyword.control.conditional)

(range_action
  [
    "range"
    "else"
    "end"
  ] @keyword.control.conditional)

(template_action
  "template" @function.builtin)

(block_action
  [
    "block"
    "end"
  ] @keyword.directive)

(define_action
  [
    "define"
    "end"
  ] @keyword.directive)

(with_action
  [
    "with"
    "else"
    "end"
  ] @keyword.control.conditional)

; Literals

[
  (interpreted_string_literal)
  (raw_string_literal)
  (rune_literal)
] @string

(escape_sequence) @string.special

[
  (int_literal)
  (imaginary_literal)
] @constant.numeric.integer

(float_literal) @constant.numeric.float

[
  (true)
  (false)
] @constant.builtin.boolean

(nil) @constant.builtin

(comment) @comment
