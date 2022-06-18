[
  "/dts-v1/"
  "/memreserve/"
  "/delete-node/"
  "/delete-property/"
] @keyword

[
  "#define"
  "#include"
] @keyword.directive

[
  "!"
  "~"
  "-"
  "+"
  "*"
  "/"
  "%"
  "||"
  "&&"
  "|"
  "^"
  "&"
  "=="
  "!="
  ">"
  ">="
  "<="
  ">"
  "<<"
  ">>"
] @operator

[
  ","
  ";"
] @punctuation.delimiter

[
  "("
  ")"
  "{"
  "}"
  "<"
  ">"
] @punctuation.bracket

(string_literal) @string

(integer_literal) @constant.numeric.integer

(call_expression
  function: (identifier) @function)

(labeled_item
  label: (identifier) @label)

(identifier) @variable

(unit_address) @tag

(reference) @constant

(comment) @comment
