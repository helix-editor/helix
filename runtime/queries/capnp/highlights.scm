; Preproc

[
  (unique_id)
  (top_level_annotation_body)
] @keyword.directive

; Includes

[
  "import"
  "$import"
  "embed"
  "using"
] @keyword.control.import

(import_path) @string

; Builtins

[
  (primitive_type)
  "List"
] @type.builtin

; Typedefs

(type_definition) @type

; Labels (@number, @number!)

(field_version) @label

; Methods

(annotation_definition_identifier) @function.method
(method_identifier) @function.method

; Fields

(field_identifier) @variable.other.member

; Properties

(property) @label

; Parameters

(param_identifier) @variable.parameter
(return_identifier) @variable.parameter

; Constants

(const_identifier) @variable
(local_const) @constant
(enum_member) @type.enum.variant

(void) @constant.builtin

; Types

(enum_identifier) @type.enum
(extend_type) @type
(type_identifier) @type

; Attributes

(annotation_identifier) @attribute
(attribute) @attribute

; Operators

[
 ; @ ! -
  "="
] @operator

; Keywords


[
  "annotation"
  "enum"
  "group"
  "interface"
  "struct"
  "union"
] @keyword.storage.type

"extends" @keyword

[
  "namespace"
  (annotation_target)
] @special

; Literals

[
  (string)
  (concatenated_string)
  (block_text)
  (namespace)
] @string

(escape_sequence) @constant.character.escape

(data_string) @string.special

(number) @constant.numeric.integer

(float) @constant.numeric.float

(boolean) @constant.builtin.boolean

; Misc

[
  "const"
] @keyword.storage.modifier

[
  "*"
  "$"
  ":"
] @string.special.symbol

["{" "}"] @punctuation.bracket

["(" ")"] @punctuation.bracket

["[" "]"] @punctuation.bracket

[
  ","
  ";"
  "->"
] @punctuation.delimiter

(data_hex) @constant

; Comments

(comment) @comment.line
