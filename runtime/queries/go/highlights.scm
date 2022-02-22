; Function calls

(call_expression
  function: (identifier) @function)

(call_expression
  function: (selector_expression
    field: (field_identifier) @function.method))

; Function definitions

(function_declaration
  name: (identifier) @function)

(method_declaration
  name: (field_identifier) @function.method)

; Identifiers

((identifier) @constant (match? @constant "^[A-Z][A-Z\\d_]+$"))
(const_spec
  name: (identifier) @constant)

(parameter_declaration (identifier) @variable.parameter)
(variadic_parameter_declaration (identifier) @variable.parameter)

(type_identifier) @type
(field_identifier) @variable.other.member
(identifier) @variable
(package_identifier) @variable


; Operators

[
  "--"
  "-"
  "-="
  ":="
  "!"
  "!="
  "..."
  "*"
  "*"
  "*="
  "/"
  "/="
  "&"
  "&&"
  "&="
  "%"
  "%="
  "^"
  "^="
  "+"
  "++"
  "+="
  "<-"
  "<"
  "<<"
  "<<="
  "<="
  "="
  "=="
  ">"
  ">="
  ">>"
  ">>="
  "|"
  "|="
  "||"
  "~"
] @operator

; Keywords

[
  "break"
  "case"
  "chan"
  "const"
  "continue"
  "default"
  "defer"
  "else"
  "fallthrough"
  "for"
  "func"
  "go"
  "goto"
  "if"
  "interface"
  "map"
  "range"
  "return"
  "select"
  "struct"
  "switch"
  "type"
  "var"
] @keyword

[
  "import"
  "package"
] @keyword.control.import

; Delimiters

[
  ":"
  "."
  ","
  ";"
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

; Literals

[
  (interpreted_string_literal)
  (raw_string_literal)
  (rune_literal)
] @string

(escape_sequence) @constant.character.escape

[
  (int_literal)
  (float_literal)
  (imaginary_literal)
] @constant.numeric.integer

[
  (true)
  (false)
] @constant.builtin.boolean

[
  (nil)
  (iota)
] @constant.builtin

(comment) @comment
