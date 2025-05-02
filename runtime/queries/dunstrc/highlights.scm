(assign
  (key) @attribute)

(assign
  (value
    (quoted) @string))

(comment) @comment.line

[
  "["
  "]"
] @punctuation.bracket

"=" @operator

(section
  (name) @namespace)
