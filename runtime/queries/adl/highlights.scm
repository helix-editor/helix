; adl

[
"module"
"struct"
"union"
"type"
"newtype"
"annotation"
] @keyword

(scoped_name) @variable
(comment) @comment
(doc_comment) @info
(name) @type

(fname) @property

(type_expr (scoped_name) @type)

(type_expr (scoped_name) @generic (type_param) @type.parameter)

; json
(key) @string.special.key

(string) @string

(number) @constant.numeric

[
  (null)
  (true)
  (false)
] @constant.builtin

(escape_sequence) @constant.character.escape

