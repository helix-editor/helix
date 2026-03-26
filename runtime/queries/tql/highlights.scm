;; Upstream: https://github.com/tenzir/tree-sitter-tql/blob/main/queries/tql/highlights.scm

"move" @keyword

[
  "let"
] @keyword.storage

[
  "and"
  "or"
  "not"
  "in"
] @keyword.operator

[
  "if"
  "else"
  "match"
] @keyword.control.conditional

"this" @variable.builtin

[
  "."
  ":"
] @punctuation.delimiter

[
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  "?"
  "..."
  "+"
  "-"
  "*"
  "/"
  "="
  "=>"
  "|"
  "::"
  "=="
  "!="
  ">"
  ">="
  "<"
  "<="
] @operator

[
  "("
  ")"
] @punctuation.bracket

[
  ","
] @punctuation.delimiter

"_" @variable.builtin

"null" @constant.builtin

[
  "true"
  "false"
] @constant.builtin.boolean

(dollar_var) @variable
(meta_selector) @attribute
(number) @constant.numeric
(string) @string
(format_expr) @string
(ip) @constant
(subnet) @constant
(time) @number
(duration) @constant.numeric
(frontmatter_open) @comment
(frontmatter_close) @comment
(comment) @comment

(invocation
  operator: (entity) @function.call)
(call_expression
  (entity) @function.call)
(call_expression
  method: (entity) @function.method)
(identifier) @variable
