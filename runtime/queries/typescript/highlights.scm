; inherits: javascript

; Types

(type_identifier) @type
(predefined_type) @type.builtin

((identifier) @type
 (#match? @type "^[A-Z]"))

(type_arguments
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

; Variables

(required_parameter (identifier) @variable.parameter)
(optional_parameter (identifier) @variable.parameter)

; Keywords

[
  "abstract"
  "declare"
  "export"
  "implements"
  "keyof"
  "namespace"
] @keyword

[
  "type"
  "interface"
  "enum"
] @keyword.storage.type

[
  "public"
  "private"
  "protected"
  "readonly"
] @keyword.storage.modifier