; adl

[
"module"
"struct"
"union"
"type"
"newtype"
"annotation"
] @keyword

(adl (scoped_name)) @namespace
(comment) @comment
(doc_comment) @comment.block.documentation
(name) @type

(fname) @variable.other.member

(type_expr (scoped_name) @type)

(type_expr_params (param (scoped_name) @type.parameter))

; json
(key) @string.special

(string) @string

(number) @constant.numeric

[
  (null)
  (true)
  (false)
] @constant.builtin

(escape_sequence) @constant.character.escape

