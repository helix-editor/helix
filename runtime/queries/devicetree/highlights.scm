[
  "/dts-v1/"
  "/memreserve/"
  "/delete-node/"
  "/delete-property/"
  "/omit-if-no-ref/"
  "/incbin/"
  "/include/"
  "/bits/"
  "/plugin/"
] @keyword

[
  "#define"
  "#include"
  "#undef"
  "#if"
  "#ifdef"
  "#ifndef"
  "#elif"
  "#elifdef"
  "#elifndef"
  "#else"
  "#endif"
  "defined"
] @keyword.directive

[
  "!"
  "~"
  "-"
  "+"
  "*"
  "/"
  "%"
  "||"
  "&&"
  "|"
  "^"
  "&"
  "=="
  "!="
  ">"
  ">="
  "<="
  ">"
  "<<"
  ">>"
  "="
  "?"
] @operator

[
  ","
  ";"
  ":"
] @punctuation.delimiter

[
  "("
  ")"
  "{"
  "}"
  "<"
  ">"
] @punctuation.bracket

(string_literal) @string
(byte_string_literal) @string

(integer_literal) @constant.numeric.integer

(identifier) @variable

(call_expression
  function: (identifier) @function)
(preproc_function_def
  name: (identifier) @function.special)

(node
  label: (identifier) @label)
(property
  label: (identifier) @label)
(memory_reservation
  label: (identifier) @label)

(property
  name: (identifier) @property)

(unit_address) @tag

; Phandle references (`&label`, `&{/path}`): colour the whole reference as a
; constant. The label identifier needs its own capture to reclaim it from the
; `(identifier) @variable` base — helix lets the contained node win otherwise.
(reference) @constant
(reference (identifier) @constant)

(comment) @comment
