; Keywords
[
  "module"
  "using"
  "single"
  "multi"
  "link"
  "property"
  "constraint"
  "tuple"
  "annotation"
  "abstract"
  "scalar"
  "type"
  "required"
  "optional"
  "extension"
  "function"
] @keyword

(modifier) @keyword
(extending) @keyword

(module name: (identifier) @namespace)
(object_type) @type

(comment) @comment

; Properties
(property) @variable.other.member
(link) @variable.other.member
(annotation) @variable.other.member

(identifier) @variable
(string) @string
(edgeql_fragment) @string
; Builtins

(type) @type
[
  "str"
  "bool"
  "int16"
  "int32"
  "int64"
  "float32"
  "float64"
  "bigint"
  "decimal"
  "json"
  "uuid"
  "bytes"
  "datetime"
  "duration"
  "sequence"
  "anytype"
] @type.builtin

(true) @constant.builtin
(false) @constant.builtin
(null) @constant.builtin

; Delimiters
[
  ";"
  ","
] @punctuation.delimiter

; Operators
[
  "->"
  ":="
] @operator

