[
  (module)
  (as)
  (exposing)
  (backslash)
] @keyword

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
  (infix)
  (port)
  "let"
  "in"
] @keyword.storage.type

(dot) @operator

[
  (colon)
  (arrow)
  (eq)
  (operator_identifier)
  "|"
] @keyword.operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

"," @punctuation.delimiter

; modules

(module_declaration(upper_case_qid) @namespace)
(import_clause(upper_case_qid) @namespace)
(import_clause(as_clause(upper_case_identifier) @namespace))
(exposing_list(exposed_type(upper_case_identifier) @type))
(exposing_list(exposed_value) @variable)

; functions

(type_annotation(lower_case_identifier) @function)
(port_annotation(lower_case_identifier) @function)
(file (value_declaration (function_declaration_left(lower_case_identifier) @function)))

; types

(field name: (lower_case_identifier) @variable.other.member)
(field_type name: (lower_case_identifier) @variable.other.member)
(field_access_expr(lower_case_identifier) @variable)

(type_declaration(upper_case_identifier) @type)
(type_declaration typeName: (lower_type_name) @type.parameter)

(type_alias_declaration name: (upper_case_identifier) @type)
(type_alias_declaration typeVariable: (lower_type_name) @type.parameter)

(type_ref(upper_case_qid) @type)
(type_ref(upper_case_qid(upper_case_identifier) @namespace (dot) (upper_case_identifier) @type))

(type_variable(lower_case_identifier) @type.parameter)

; variables

(union_pattern constructor: (upper_case_qid (upper_case_identifier) @namespace (dot) (upper_case_identifier) @constructor)) 
(union_pattern constructor: (upper_case_qid (upper_case_identifier) @constructor)) 

(union_variant(upper_case_identifier) @constructor)

(value_expr name: (value_qid (upper_case_identifier) @namespace))
(value_expr(upper_case_qid(upper_case_identifier) @namespace (dot) (upper_case_identifier) @constructor))
(value_expr(upper_case_qid(upper_case_identifier)) @constructor)

(value_expr(value_qid(upper_case_identifier) @namespace (dot) (lower_case_identifier) @variable))
(value_expr(value_qid(lower_case_identifier) @variable))

(let_in_expr(value_declaration(function_declaration_left(lower_case_identifier) @variable)))

(function_declaration_left(lower_pattern(lower_case_identifier) @variable.parameter))

; comments

(line_comment) @comment
(block_comment) @comment

; numbers

(number_constant_expr) @constant.numeric

; strings

(string_escape) @constant.character.escape

(string_constant_expr) @string
(char_constant_expr) @constant.character
