(comment) @comment

; Different types:
(string_value) @string
(bool_value) @constant.builtin.boolean

; Constants

(escape_sequence) @constant.character.escape

(color_value) @constant

[
  (children_identifier)
  (easing_kind_identifier)
] @constant.builtin

[
  (int_value)
  (physical_length_value)
] @constant.numeric.integer

[
  (float_value)
  (percent_value)
  (length_value)
  (duration_value)
  (angle_value)
  (relative_font_size_value)
] @constant.numeric.float

(purity) @keyword.storage.modifier

(function_visibility) @keyword.storage.modifier

(property_visibility) @keyword.storage.modifier

(builtin_type_identifier) @type.builtin

(reference_identifier) @variable.builtin

(type
  [
    (type_list)
    (user_type_identifier)
    (anon_struct_block)
  ]) @type

(user_type_identifier) @type

; Functions and callbacks
(argument) @variable.parameter

(function_call
  name: (_) @function.call)

; definitions
(callback
  name: (_) @function)

(callback_alias
  name: (_) @function)

(callback_event
  name: (simple_identifier) @function.call)

(enum_definition
  name: (_) @type.enum)

(function_definition
  name: (_) @function)

(struct_definition
  name: (_) @type)

(typed_identifier
  type: (_) @type)

; Operators
(binary_expression
  op: (_) @operator)

(unary_expression
  op: (_) @operator)

[
  (comparison_operator)
  (mult_prec_operator)
  (add_prec_operator)
  (unary_prec_operator)
  (assignment_prec_operator)
] @operator

[
  ":="
  "=>"
  "->"
  "<=>"
] @operator

[
  ";"
  "."
  ","
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

(property
  [
    "<"
    ">"
  ] @punctuation.bracket)

; Properties, constants and variables
(component
  id: (simple_identifier) @constant)

(property
  name: (simple_identifier) @variable)

(binding_alias
  name: (simple_identifier) @variable)

(binding
  name: (simple_identifier) @variable)

(struct_block
  (simple_identifier) @variable.other.member)

(anon_struct_block
  (simple_identifier) @variable.other.member)

(property_assignment
  property: (simple_identifier) @variable)

(states_definition
  name: (simple_identifier) @variable)

(callback
  name: (simple_identifier) @variable)

(typed_identifier
  name: (_) @variable)

(simple_indexed_identifier
  (simple_identifier) @variable)

(expression
  (simple_identifier) @variable)

; Attributes
[
  (linear_gradient_identifier)
  (radial_gradient_identifier)
  (radial_gradient_kind)
] @attribute

(image_call
  "@image-url" @attribute)

(tr
  "@tr" @attribute)

; Keywords
(animate_option_identifier) @keyword

(export) @keyword.control.import

(if_statement
  "if" @keyword.control.conditional)

(if_expr
  [
    "if"
    "else"
  ] @keyword.control.conditional)

(ternary_expression
  [
    "?"
    ":"
  ] @keyword.control.conditional)

(animate_statement
  "animate" @keyword)

(callback
  "callback" @keyword.function)

(component_definition
  [
    "component"
    "inherits"
  ] @keyword.storage.type)

(enum_definition
  "enum" @keyword.storage.type)

(for_loop
  [
    "for"
    "in"
  ] @keyword.control.repeat)

(function_definition
  "function" @keyword.function)

(global_definition
  "global" @keyword.storage.type)

(imperative_block
  "return" @keyword.control.return)

(import_statement
  [
    "import"
    "from"
  ] @keyword.control.import)

(import_type
  "as" @keyword.control.import)

(property
  "property" @keyword.storage.type)

(states_definition
  [
    "states"
    "when"
  ] @keyword)

(struct_definition
  "struct" @keyword.storage.type)

(transitions_definition
  [
    "transitions"
    "in"
    "out"
  ] @keyword)
