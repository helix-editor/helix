(ERROR) @error

((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]+$"))
((identifier_def) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]+$"))

((identifier) @namespace
  (#match? @namespace "^[A-Z]"))
((identifier_def) @namespace
  (#match? @namespace "^[A-Z]"))

(identifier "." @punctuation)
(function_call (identifier) @function)
(func (identifier_def) @function)

(string) @string
(atom_short_string) @string

(code_element_directive) @keyword.directive
"return" @keyword

(number) @constant.numeric
(atom_hex_number) @constant.numeric

(comment) @comment

"*" @special
(type) @type

[
  "felt"
  ; "codeoffset"
] @type.builtin

[
  "if"
  "else"
  "end"
  "assert"
  "with"
  "with_attr"
] @keyword.control

[
  "from"
  "import"
  "func"
  "namespace"
] @keyword ; keyword.declaration

[
  "let"
  "const"
  "local"
  "struct"
  "member"
  "alloc_locals"
  "tempvar"
] @keyword

(decorator) @attribute

[
  "="
  "+"
  "-"
  "*"
  "/"
  ; "%"
  ; "!"
  ; ">"
  ; "<"
  ; "\\"
  ; "&"
  ; "?"
  ; "^"
  ; "~"
  "=="
  "!="
  "new"
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  ","
  ":"
] @punctuation.delimiter
