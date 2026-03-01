;; Simple tokens
(terminal) @string

(special_sequence) @string.special

(integer) @number

(comment) @comment.block

;; Identifiers
(identifier) @identifier

;; Punctuation
[
 ";"
 ","
] @punctuation.delimiter

[
 "|"
 "*"
 "-"
] @operator

"=" @keyword.operator

[
 "("
 ")"
 "["
 "]"
 "{"
 "}"
] @punctuation.bracket
