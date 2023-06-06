; Namespaces

(internal_module
  [((identifier) @namespace) ((nested_identifier (identifier) @namespace))])

(ambient_declaration "global" @namespace)

; Variables

(required_parameter (identifier) @variable.parameter)
(optional_parameter (identifier) @variable.parameter)

; Punctuation

[
  ":"
] @punctuation.delimiter

(optional_parameter "?" @punctuation.special)
(property_signature "?" @punctuation.special)

(conditional_type ["?" ":"] @operator)

; Keywords

[
  "abstract"
  "declare"
  "export"
  "infer"
  "implements"
  "keyof"
  "namespace"
  "override"
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

; Types

(type_identifier) @type
(predefined_type) @type.builtin

(type_arguments
  [
    "<"
    ">"
  ] @punctuation.bracket)

(type_parameters
  [
    "<"
    ">"
  ] @punctuation.bracket)

((identifier) @type
 (#match? @type "^[A-Z]"))

; Literals

[
  (template_literal_type)
] @string

; Tokens

(template_type
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

; inherits: ecma
