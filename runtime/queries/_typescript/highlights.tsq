; Namespaces
; ----------

(internal_module
  [((identifier) @namespace) ((nested_identifier (identifier) @namespace))])

(ambient_declaration "global" @namespace)

; Parameters
; ----------
; Javascript and Typescript Treesitter grammars deviate when defining the
; tree structure for parameters, so we need to address them in each specific
; language instead of ecma.

; (p: t)
; (p: t = 1)
(required_parameter 
  (identifier) @variable.parameter)

; (...p: t)
(required_parameter
  (rest_pattern
    (identifier) @variable.parameter))

; ({ p }: { p: t })
(required_parameter
  (object_pattern
    (shorthand_property_identifier_pattern) @variable.parameter))

; ({ a: p }: { a: t })
(required_parameter
  (object_pattern
    (pair_pattern
      value: (identifier) @variable.parameter)))

; ([ p ]: t[])
(required_parameter
  (array_pattern
    (identifier) @variable.parameter))

; (p?: t)
; (p?: t = 1) // Invalid but still possible to highlight.
(optional_parameter 
  (identifier) @variable.parameter)

; (...p?: t) // Invalid but still possible to highlight.
(optional_parameter
  (rest_pattern
    (identifier) @variable.parameter))

; ({ p }: { p?: t})
(optional_parameter
  (object_pattern
    (shorthand_property_identifier_pattern) @variable.parameter))

; ({ a: p }: { a?: t })
(optional_parameter
  (object_pattern
    (pair_pattern
      value: (identifier) @variable.parameter)))

; ([ p ]?: t[]) // Invalid but still possible to highlight.
(optional_parameter
  (array_pattern
    (identifier) @variable.parameter))

; Punctuation
; -----------

[
  ":"
] @punctuation.delimiter

(optional_parameter "?" @punctuation.special)
(property_signature "?" @punctuation.special)

(conditional_type ["?" ":"] @operator)

; Keywords
; --------

[
  "abstract"
  "declare"
  "export"
  "infer"
  "implements"
  "keyof"
  "namespace"
  "override"
  "satisfies"
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
; -----

(type_parameter
  name: (type_identifier) @type.parameter)
(type_identifier) @type
(predefined_type) @type.builtin

; Type arguments and parameters
; -----------------------------

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

; Literals
; --------

[
  (template_literal_type)
] @string

; Tokens
; ------

(template_type
  "${" @punctuation.special
  "}" @punctuation.special) @embedded
