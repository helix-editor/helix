; highlights.scm

[
  "definition"
  "caveat"
  "permission"
  "relation"
  "nil"
] @keyword

[
  ","
  ":"
] @punctuation.delimiter

[
  "("
  ")"
  "{"
  "}"
] @punctuation.bracket

[
  "|"
  "+"
  "-"
  "&"
  "#"
  "->"
  "="
] @operator
("with") @keyword.operator

[
  "nil"
  "*"
] @constant.builtin

(comment) @comment
(type_identifier) @type
(cel_type_identifier) @type
(cel_variable_identifier) @variable.parameter
(field_identifier) @variable.other.member
[
  (func_identifier)
  (method_identifier)
] @function.method
