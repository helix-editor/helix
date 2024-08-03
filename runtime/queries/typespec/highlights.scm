; Keywords

[
  "is"
  "extends"
  "valueof"
] @keyword.operator

[
  "namespace"
  "scalar"
  "interface"
  "alias"
] @keyword

[
  "model"
  "enum"
  "union"
] @keyword.storage.type

[
  "op"
  "fn"
  "dec"
] @keyword.function

"extern" @keyword.storage.modifier

[
  "import"
  "using"
] @keyword.control.import

[
  "("
  ")"
  "{"
  "}"
  "<"
  ">"
  "["
  "]"
] @punctuation.bracket

[
  ","
  ";"
  "."
  ":"
] @punctuation.delimiter

[
  "|"
  "&"
  "="
  "..."
] @operator

"?" @punctuation.special

; Imports

(import_statement
  (quoted_string_literal) @string.special.path)

; Namespaces

(using_statement
  module: (identifier_or_member_expression) @namespace)

(namespace_statement
  name: (identifier_or_member_expression) @namespace)

; Comments

[
  (single_line_comment)
] @comment.line

[
  (multi_line_comment)
] @comment.block

; Decorators

(decorator
  "@" @attribute
  name: (identifier_or_member_expression) @attribute)

(augment_decorator_statement
  name: (identifier_or_member_expression) @attribute)

(decorator
  (decorator_arguments) @variable.parameter)

; Scalars

(scalar_statement
  name: (identifier) @type)

; Models

(model_statement
  name: (identifier) @type)

(model_property
  name: (identifier) @variable.other.member)

; Operations

(operation_statement
  name: (identifier) @function.method)

(operation_arguments
  (model_property
    name: (identifier) @variable.parameter))

(template_parameter
  name: (identifier) @type.parameter)

(function_parameter
  name: (identifier) @variable.parameter)

; Interfaces

(interface_statement
  name: (identifier) @type)

(interface_statement
  (interface_body
    (interface_member
      (identifier) @function.method)))

; Enums

(enum_statement
  name: (identifier) @type.enum)

(enum_member
  name: (identifier) @constant)

; Unions

(union_statement
  name: (identifier) @type)

(union_variant
  name: (identifier) @type.enum.variant)

; Aliases

(alias_statement
  name: (identifier) @type)

; Built-in types

[
  (quoted_string_literal)
  (triple_quoted_string_literal)
] @string

(escape_sequence) @constant.character.escape

(boolean_literal) @constant.builtin.boolean

[
  (decimal_literal)
  (hex_integer_literal)
  (binary_integer_literal)
] @constant.numeric.integer

(builtin_type) @type.builtin

; Identifiers

(identifier_or_member_expression) @type
