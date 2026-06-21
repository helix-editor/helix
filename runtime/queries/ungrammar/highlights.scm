(comment) @comment

; The node name on the left of `=` being defined.
(definition) @type

; References to other rules.
(identifier) @variable

(label_name) @label

(token) @string

[
  "="
  "|"
  "?"
  "*"
] @operator

":" @punctuation.delimiter

[
  "("
  ")"
] @punctuation.bracket
