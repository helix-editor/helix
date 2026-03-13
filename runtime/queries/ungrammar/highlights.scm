(line_comment) @comment

(identifier) @function

(labeled_rule
  (identifier) @type)

(node_rule
  (identifier) @variable.parameter)

(token) @string

[
  "="
  "|"
  ":"
  "("
  ")"
  "?"
  "*"
] @operator
