[
  (module)
  (backslash)
  (as)
  (port)
  (exposing)
  (infix)
  "|"
] @keyword

[
  "let"
  "in"
] @keyword.control

(import) @keyword.control.import

[
  "if"
  "then"
  "else"
  (when)
  (is)
] @keyword.control.conditional

[
  (type)
  (alias)
] @keyword.storage.type

[
  (colon)
  (arrow)
  (dot)
  (operator_identifier)
] @operator

(eq) @keyword.operator.assignment

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

"," @punctuation.delimiter

; functions

(type_annotation(lower_case_identifier) @function)
(port_annotation(lower_case_identifier) @function)
(file (value_declaration (function_declaration_left(lower_case_identifier) @function)))

; types

(field name: (lower_case_identifier) @variable.other.member)
(field_type name: (lower_case_identifier) @variable.other.member)
(field_access_expr(lower_case_identifier) @variable.other.member)

(type_declaration(upper_case_identifier) @type)
(type_declaration typeName: (lower_type_name (lower_case_identifier)) @type.parameter)
((type_ref) @type)
(type_alias_declaration name: (upper_case_identifier) @type)
(type_alias_declaration typeVariable: (lower_type_name (lower_case_identifier)) @type.parameter)
(type_variable (lower_case_identifier) @type.parameter)

(union_pattern constructor: (upper_case_qid (upper_case_identifier) @label (dot) (upper_case_identifier) @variable.other.member)) 
(union_pattern constructor: (upper_case_qid (upper_case_identifier) @variable.other.member)) 

(union_variant(upper_case_identifier) @variable.other.member)
(value_expr name: (value_qid (upper_case_identifier) @label))
(value_expr (upper_case_qid (upper_case_identifier) @label (dot) (upper_case_identifier) @variable.other.member))
(value_expr(upper_case_qid(upper_case_identifier)) @variable.other.member)

; comments
(line_comment) @comment
(block_comment) @comment

; numbers
(number_constant_expr) @constant.numeric

; strings
(string_escape) @constant.character.escape

(string_constant_expr) @string
(char_constant_expr) @constant.character
