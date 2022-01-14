; Types
;------

(scalar_type_definition
  (name) @type)

(object_type_definition
  (name) @type)

(interface_type_definition
  (name) @type)

(union_type_definition
  (name) @type)

(enum_type_definition
  (name) @type)

(input_object_type_definition
  (name) @type)

(directive_definition
  (name) @type)

(directive_definition
  "@" @type)

(scalar_type_extension
  (name) @type)

(object_type_extension
  (name) @type)

(interface_type_extension
  (name) @type)

(union_type_extension
  (name) @type)

(enum_type_extension
  (name) @type)

(input_object_type_extension
  (name) @type)

(named_type
  (name) @type)

(directive) @type

; Properties
;-----------

(field
  (name) @label)

(field
  (alias
    (name) @label))

(field_definition
  (name) @label)

(object_value
  (object_field
    (name) @label))

(enum_value
  (name) @label)

; Variable Definitions and Arguments 
;-----------------------------------

(operation_definition
  (name) @variable)

(fragment_name
  (name) @variable)

(input_fields_definition
  (input_value_definition
    (name) @variable.parameter))

(argument
  (name) @variable.parameter)

(arguments_definition
  (input_value_definition
    (name) @variable.parameter))

(variable_definition
  (variable) @variable.parameter)

(argument
  (value
    (variable) @variable))

; Constants
;----------

(string_value) @string

(int_value) @constants.numeric.integer

(float_value) @constants.numeric.float

(boolean_value) @constants.builtin.boolean

; Literals
;---------

(description) @comment

(comment) @comment

(directive_location
  (executable_directive_location) @type.builtin)

(directive_location
  (type_system_directive_location) @type.builtin)

; Keywords
;----------

[
  "query"
  "mutation"
  "subscription"
  "fragment"
  "scalar"
  "type"
  "interface"
  "union"
  "enum"
  "input"
  "extend"
  "directive"
  "schema"
  "on"
  "repeatable"
  "implements"
] @keyword

; Punctuation
;------------

[
 "("
 ")"
 "["
 "]"
 "{"
 "}"
] @punctuation.bracket

"=" @operator

"|" @punctuation.delimiter
"&" @punctuation.delimiter
":" @punctuation.delimiter

"..." @punctuation.special
"!" @punctuation.special
