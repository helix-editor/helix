"module" @keyword.other

[
  "let"
  "rec"
  "if"
  "else"
  "match"
  "with"
] @keyword.control

[
  "="
] @keyword.type

[
  "static"
  "member"
] @keyword.other

(identifier) @type

[
  (type_definition)
  (union_type_defn)
  (match_expression)
  (function_or_value_defn)
  (function_declaration_left)
  (value_declaration_left)
] @keyword.type

(application_expression) @keyword.other

