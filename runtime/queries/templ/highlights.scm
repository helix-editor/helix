(package_identifier) @namespace

(parameter_declaration (identifier) @variable.parameter)
(variadic_parameter_declaration (identifier) @variable.parameter)

(function_declaration
  name: (identifier) @function)

(type_spec name: (type_identifier) @type)
(type_identifier) @type
(field_identifier) @variable.other.member
(identifier) @variable

; Function calls

(call_expression
  function: (identifier) @function)

(call_expression
  function: (selector_expression
    field: (field_identifier) @function))

;
; These are Templ specific
;

(component_declaration
  name: (component_identifier) @function)

(tag_start) @tag
(tag_end) @tag
(self_closing_tag) @tag
(style_element) @tag

(attribute
  name: (attribute_name) @attribute)
(attribute
  value: (quoted_attribute_value) @string)

(element_text) @string.special
(style_element_text) @string.special

(css_property
  name: (css_property_name) @attribute)

(expression) @function.method
(dynamic_class_attribute_value) @function.method

(component_import
  name: (component_identifier) @function)

(component_render) @function

[
  "@"
] @operator

[
  "func"
  "var"
  "const"
  "templ"
  "css"
  "type"
  "struct"
  "range"
  "script"
] @keyword.storage.type

[
  "return"
] @keyword.control.return

[
  "import"
  "package"
] @keyword.control.import

[
  "else"
  "case"
  "switch"
  "if"
  "default"
] @keyword.control.conditional

"for" @keyword.control.repeat

[
  (interpreted_string_literal)
  (raw_string_literal)
  (rune_literal)
] @string

; Comments

(comment) @comment

(element_comment) @comment
