; Queries are taken from: https://github.com/indoorvivants/tree-sitter-smithy/blob/main/queries/highlights.scm
; Preproc
(control_key) @preproc

; Namespace
(namespace) @namespace

; Includes
[
  "use"
] @include

; Builtins
(primitive) @type.builtin
[
  "enum"
  "intEnum"
  "list"
  "map"
  "set"
] @type.builtin

; Fields (Members)
; (field) @field

(key_identifier) @field
(shape_member
  (field) @field)
(operation_field) @field
(operation_error_field) @field

; Constants
(enum_member
  (enum_field) @constant)

; Types
(identifier) @type
(structure_resource
  (shape_id) @type)

; Attributes
(mixins
  (shape_id) @attribute)
(trait_statement
  (shape_id (#set! "priority" 105)) @attribute)

; Operators
[
  "@"
  "-"
  "="
  ":="
] @operator

; Keywords
[
  "namespace"
  "service"
  "structure"
  "operation"
  "union"
  "resource"
  "metadata"
  "apply"
  "for"
  "with"
] @keyword

; Literals
(string) @string
(escape_sequence) @string.escape

(number) @number

(float) @float

(boolean) @boolean

(null) @constant.builtin

; Misc
[
  "$"
  "#"
] @punctuation.special

["{" "}"] @punctuation.bracket

["(" ")"] @punctuation.bracket

["[" "]"] @punctuation.bracket

[
  ":"
  "."
] @punctuation.delimiter

; Comments
[
  (comment)
  (documentation_comment)
] @comment @spell
