; Includes
[
  "import"
  "provider"
  "with"
  "as"
  "from"
] @keyword.control.import

; Namespaces
(module_declaration
  (identifier) @namespace)

; Builtins
(primitive_type) @type.builtin

((member_expression
  object: (identifier) @type.builtin)
  (#eq? @type.builtin "sys"))

; Functions
(call_expression
  function: (identifier) @function)

(user_defined_function
  name: (identifier) @function)

; Properties
(object_property
  (identifier) @function.method
  ":" @punctuation.delimiter
  (_))

(object_property
  (compatible_identifier) @function.method
  ":" @punctuation.delimiter
  (_))

(property_identifier) @function.method

; Attributes
(decorator
  "@" @attribute)

(decorator
  (call_expression
    (identifier) @attribute))

(decorator
  (call_expression
    (member_expression
      object: (identifier) @attribute
      property: (property_identifier) @attribute)))

; Types
(type_declaration
  (identifier) @type)

(type_declaration
  (identifier)
  "="
  (identifier) @type)

(type
  (identifier) @type)

(resource_declaration
  (identifier) @type)

(resource_expression
  (identifier) @type)

; Parameters
(parameter_declaration
  (identifier) @variable.parameter
  (_))

(call_expression
  function: (_)
  (arguments
    (identifier) @variable.parameter))

(call_expression
  function: (_)
  (arguments
    (member_expression
      object: (identifier) @variable.parameter)))

(parameter
  .
  (identifier) @variable.parameter)

; Variables
(variable_declaration
  (identifier) @variable
  (_))

(metadata_declaration
  (identifier) @variable
  (_))

(output_declaration
  (identifier) @variable
  (_))

(object_property
  (_)
  ":"
  (identifier) @variable)

(for_statement
  "for"
  (for_loop_parameters
    (loop_variable) @variable
    (loop_enumerator) @variable))

; Conditionals
"if" @keyword.conditional

(ternary_expression
  "?" @keyword.control.conditional
  ":" @keyword.control.conditional)

; Loops
(for_statement
  "for" @keyword.control.repeat
  "in"
  ":" @punctuation.delimiter)

; Keywords
[
  "module"
  "metadata"
  "output"
  "param"
  "resource"
  "existing"
  "targetScope"
  "type"
  "var"
  "using"
  "test"
] @keyword

"func" @keyword.function

"assert" @keyword.control.exception

; Operators
[
  "+"
  "-"
  "*"
  "/"
  "%"
  "||"
  "&&"
  "|"
  "=="
  "!="
  "=~"
  "!~"
  ">"
  ">="
  "<="
  "<"
  "??"
  "="
  "!"
  ".?"
] @operator

(subscript_expression
  "?" @operator)

(nullable_type
  "?" @operator)

"in" @keyword.operator

; Literals
(string) @string

(escape_sequence) @constant.character

(number) @constant.number

(boolean) @constant.builtin.boolean

(null) @constant.builtin

; Misc
(compatible_identifier
  "?" @punctuation.special)

(nullable_return_type) @punctuation.special

[
  "{"
  "}"
] @punctuation.bracket

[
  "["
  "]"
] @punctuation.bracket

[
  "("
  ")"
] @punctuation.bracket

[
  "."
  ":"
  "::"
  "=>"
] @punctuation.delimiter

; Interpolation
(interpolation
  "${" @punctuation.special
  "}" @punctuation.special)

(interpolation
  (identifier) @variable)

; Comments
[
  (comment)
  (diagnostic_comment)
] @comment
