; Literals
; --------

(boolean) @constant.builtin.boolean
(comment) @comment
(string) @string
(number) @constant.numeric
(null) @constant.builtin

; Punctuation
; -----------

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket

[
  "."
  ","
] @punctuation.delimiter

[
  "="
] @operator

; Function definitions
;---------------------

(function
  name: (identifier) @function)


(attribute (identifier) @variable.other.member)
(block (identifier) @type.builtin)
