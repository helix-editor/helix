; Preproc

(unique_id) @preproc
(top_level_annotation_body) @preproc

; Includes

[
  "import"
  "$import"
  "embed"
] @include

(import_path) @string

; Builtins

[
  (primitive_type)
  "List"
] @type.builtin

; Typedefs

(type_definition) @type.definition

; Labels (@number, @number!)

(field_version) @label

; Methods

(annotation_definition_identifier) @method
(method_identifier) @method

; Fields

(field_identifier) @field

; Properties

(property) @property

; Parameters

(param_identifier) @parameter
(return_identifier) @parameter

; Constants

(const_identifier) @constant
(local_const) @constant
(enum_member) @constant

(void) @constant.builtin

; Types

(enum_identifier) @type
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
] @keyword

[
  "extends"
  "namespace"
  "using"
  (annotation_target)
] @keyword

; Literals

[
  (string)
  (concatenated_string)
  (block_text)
  (namespace)
] @string

(escape_sequence) @string.escape

(data_string) @string.special

(number) @number

(float) @float

(boolean) @boolean

; Misc

[
  "const"
] @type.qualifier

[
  "*"
  "$"
  ":"
] @punctuation.special

["{" "}"] @punctuation.bracket

["(" ")"] @punctuation.bracket

["[" "]"] @punctuation.bracket

[
  ","
  ";"
  "->"
] @punctuation.delimiter

(data_hex) @symbol

; Comments

(comment) @comment @spell
