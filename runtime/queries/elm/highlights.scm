; Keywords
[
    "if"
    "then"
    "else"
    "let"
    "in"
 ] @keyword.control
(case) @keyword.control
(of) @keyword.control

(colon) @keyword.operator
(backslash) @keyword
(as) @keyword
(port) @keyword
(exposing) @keyword
(alias) @keyword
(infix) @keyword

(arrow) @keyword.operator
(dot) @keyword.operator

(port) @keyword

(type_annotation(lower_case_identifier) @function)
(port_annotation(lower_case_identifier) @function)
(file (value_declaration (function_declaration_left(lower_case_identifier) @function)))

(field name: (lower_case_identifier) @attribute)
(field_access_expr(lower_case_identifier) @attribute)

(operator_identifier) @keyword.operator
(eq) @keyword.operator.assignment

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

"|" @keyword
"," @punctuation.delimiter

[
  "|>"
] @keyword


(import) @keyword.control.import
(module) @keyword.other

(number_constant_expr) @constant.numeric

(type) @type

(type_declaration(upper_case_identifier) @type)
(type_ref) @type
(type_alias_declaration name: (upper_case_identifier) @type)

(union_pattern constructor: (upper_case_qid (upper_case_identifier) @label (dot) (upper_case_identifier) @variable.other.member)) 
(union_pattern constructor: (upper_case_qid (upper_case_identifier) @variable.other.member)) 

(union_variant(upper_case_identifier) @variable.other.member)
(value_expr name: (value_qid (upper_case_identifier) @label))
(value_expr (upper_case_qid (upper_case_identifier) @label (dot) (upper_case_identifier) @variable.other.member))
(value_expr(upper_case_qid(upper_case_identifier)) @variable.other.member)

; comments
(line_comment) @comment
(block_comment) @comment

; strings
(string_escape) @constant.character.escape

(open_quote) @string
(close_quote) @string
(regular_string_part) @string

(open_char) @constant.character
(close_char) @constant.character
