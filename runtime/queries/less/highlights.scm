; inherits: css

[
  "@import"
  "@namespace"
  "@charset"
] @keyword

(js_comment) @comment

(function_name) @function

[
  ">="
  "<="
] @operator

(plain_value) @string

(keyword_query) @function

(identifier) @variable

(variable) @variable

(arguments
  (variable) @variable.parameter)

[
  "["
  "]"
] @punctuation.bracket

(import_statement
  (identifier) @function)
