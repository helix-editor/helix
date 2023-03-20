; Queries are taken from: https://github.com/indoorvivants/tree-sitter-smithy/blob/main/queries/highlights.scm
; Preproc
(control_key) @keyword.directive

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
; (field) @variable.other.member

(key_identifier) @variable.other.member
(shape_member
  (field) @variable.other.member)
(operation_field) @variable.other.member
(operation_error_field) @variable.other.member

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
  (shape_id @attribute)

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
(escape_sequence) @constant.character.escape

(number) @number

(float) @float

(boolean) @constant.builtin.boolean

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
] @comment
