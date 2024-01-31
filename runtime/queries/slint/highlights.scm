(comment) @comment

; Different types:
(string_value) @string

(escape_sequence) @constant.character.escape

(color_value) @constant

[
  (children_identifier)
  (easing_kind_identifier)
] @constant.builtin

(bool_value) @constant.builtin.boolean

(int_value) @constant.numeric.integer

[
  (float_value)
  (percent_value)
  (length_value)
  (physical_length_value)
  (duration_value)
  (angle_value)
  (relative_font_size_value)
] @constant.numeric.float

(simple_identifier) @variable.other

(purity) @keyword.storage.modifier

(function_visibility) @keyword.storage.modifier

(property_visibility) @keyword.storage.modifier

(animate_option_identifier) @keyword

(builtin_type_identifier) @type.builtin

(reference_identifier) @variable.builtin

(type
  [
    (type_list)
    (user_type_identifier)
    (anon_struct_block)
  ]) @type

(user_type_identifier) @type

[
  (comparison_operator)
  (mult_prec_operator)
  (add_prec_operator)
  (unary_prec_operator)
  (assignment_prec_operator)
] @operator

; Functions and callbacks
(argument) @variable.parameter

(function_call) @function

; definitions
(callback
  name: (_) @function)

(component
  id: (_) @variable)

(enum_definition
  name: (_) @type.enum)

(function_definition
  name: (_) @function)

(property
  name: (_) @variable.other.member)

(struct_definition
  name: (_) @type)

(typed_identifier
  name: (_) @variable)

(typed_identifier
  type: (_) @type)

(binary_expression
  op: (_) @operator)

":=" @operator

(unary_expression
  op: (_) @operator)

(if_statement
  "if" @keyword.conditional)

(if_statement
  ":" @punctuation.delimiter)

(if_expr
  [
    "if"
    "else"
  ] @keyword.conditional)

(ternary_expression
  [
    "?"
    ":"
  ] @keyword.conditional)

; Keywords:
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

[
  (linear_gradient_identifier)
  (radial_gradient_identifier)
  (radial_gradient_kind)
] @attribute

(export) @keyword.import

(animate_option
  ":" @punctuation.delimiter)

(animate_statement
  "animate" @keyword)

(assignment_expr
  name: (_) @variable.other.member)

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
  ] @keyword.repeat)

(for_loop
  ":" @punctuation.delimiter)

(function_definition
  "function" @keyword.function)

(function_call
  name: (_) @function.call)

(global_definition
  "global" @keyword.storage.type)

(image_call
  "@image-url" @attribute)

(imperative_block
  "return" @keyword.return)

(import_statement
  [
    "import"
    "from"
  ] @keyword.import)

(import_type
  "as" @keyword.import)

(property
  [
    "property"
    "<"
    ">"
  ] @keyword.storage.type)

(states_definition
  [
    "states"
    "when"
  ] @keyword)

(struct_definition
  "struct" @keyword.storage.type)

(tr
  "@tr" @attribute)

(transitions_definition
  [
    "transitions"
    "in"
    "out"
  ] @keyword)
