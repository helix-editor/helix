; Variables

(identifier) @variable

; Includes

"include" @keyword.control.import

(include_statement (identifier) @type)
(include_statement (class_identifier (identifier) @type .))

; Keywords

[
  "class"
  "inherits"
  "node"
  "type"
  "tag"
] @keyword

[
  "define"
  "function"
] @keyword.function

[
  "if"
  "elsif"
  "else"
  "unless"
  "case"
] @keyword.control.conditional

(default_case "default" @keyword.control.conditional)

; Attributes

(attribute name: (identifier) @attribute)
(attribute name: (variable (identifier) @attribute))

; Parameters

(lambda (variable (identifier) @variable.parameter))

(parameter (variable (identifier) @variable.parameter))

(function_call (identifier) @variable.parameter)

; Functions

(function_declaration
  "function" . (identifier) @function)

(function_call
  (identifier) @function "(")

(function_call
  (field_expression "." (identifier) @function.method) "(")

(defined_resource_type
  "define" . (identifier) @function)

; Methods

(function_declaration
  "function" . (class_identifier (identifier) @function.method .))

(function_call
  (class_identifier (identifier) @function.method .))

(defined_resource_type
  "define" . (class_identifier (identifier) @function.method .))

; Types

(type) @type

(builtin_type) @type.builtin

(class_definition
  (identifier) @type)
(class_definition
  (class_identifier (identifier) @type .))

(class_inherits (identifier) @type)
(class_inherits (class_identifier (identifier) @type .))

(resource_declaration
  (identifier) @type)
(resource_declaration
  (class_identifier (identifier) @type .))

(node_definition (node_name (identifier) @type))

((identifier) @type
  (#match? @type "^[A-Z]"))

((identifier) @type.builtin
  (#any-of? @type.builtin "Boolean" "Integer" "Float" "String" "Array" "Hash" "Regexp" "Variant" "Data" "Undef" "Default" "File"))

; "Namespaces"

(class_identifier . (identifier) @namespace)

; Operators

[
  "or"
  "and"
  "in"
] @keyword.operator

[
  "="
  "+="
  "->"
  "~>"
  "<<|"
  "<|"
  "|>"
  "|>>"
  "?"
  ">"
  ">="
  "<="
  "<"
  "=="
  "!="
  "<<"
  ">>"
  "+"
  "-"
  "*"
  "/"
  "%"
  "=~"
  "!~"
] @operator

; Punctuation

[
  "|"
  "."
  ","
  ";"
  ":"
  "::"
  "=>"
] @punctuation.delimiter

[ "{" "}" ] @punctuation.bracket

[ "[" "]" ] @punctuation.bracket

[ "(" ")" ] @punctuation.bracket

(interpolation [ "${" "}" ] @punctuation.special) @none

[
  "$"
  "@"
  "@@"
] @punctuation.special

; Literals

(number) @constant.numeric.integer

(float) @constant.numeric.float

(string) @string

(escape_sequence) @constant.character.escape

(regex) @string.regexp

(boolean) @constant.builtin.boolean

[
  (undef)
  (default)
] @constant.builtin

; Comments

(comment) @comment
