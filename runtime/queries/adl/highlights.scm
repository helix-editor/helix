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

(ERROR) @error

(fname) @property

(type_expr (scoped_name) @type)

(type_expr (scoped_name) @generic (type_param) @type.param)

; json
(key) @string.special.key

(string) @string

(number) @number

[
  (null)
  (true)
  (false)
] @constant.builtin

(escape_sequence) @escape

