; Function calls

(call_expression
  function: (identifier) @function.builtin
  (#match? @function.builtin "^(append|cap|close|complex|copy|delete|imag|len|make|new|panic|print|println|real|recover)$"))

(call_expression
  function: (identifier) @function)

(call_expression
  function: (selector_expression
    field: (field_identifier) @function.method))


; Types

(type_parameter_list
  (parameter_declaration
    name: (identifier) @type.parameter))

((type_identifier) @type.builtin
  (#match? @type.builtin "^(any|bool|byte|comparable|complex128|complex64|error|float32|float64|int|int16|int32|int64|int8|rune|string|uint|uint16|uint32|uint64|uint8|uintptr)$"))

(type_identifier) @type

; Function definitions

(function_declaration
  name: (identifier) @function)

(method_declaration
  name: (field_identifier) @function.method)

(method_spec 
  name: (field_identifier) @function.method) 

; Identifiers

(const_spec
  name: (identifier) @constant)

(parameter_declaration (identifier) @variable.parameter)
(variadic_parameter_declaration (identifier) @variable.parameter)

(type_spec 
  name: (type_identifier) @constructor)
(field_identifier) @variable.other.member
(keyed_element (literal_element (identifier) @variable.other.member))
(identifier) @variable
(package_identifier) @namespace

(parameter_declaration (identifier) @variable.parameter)
(variadic_parameter_declaration (identifier) @variable.parameter)

(label_name) @label

(const_spec
  name: (identifier) @constant)

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
  "default"
  "type"
] @keyword

[
  "if"  
  "else"
  "switch"
  "select"
  "case"
] @keyword.control.conditional

[
  "for"
  "range"
] @keyword.control.repeat

[
  "import"
  "package"
] @keyword.control.import

[
  "return"
  "continue"
  "break"
  "fallthrough"
] @keyword.control.return

[
  "func"
] @keyword.function

[
  "var"
  "chan"
  "interface"
  "map"
  "struct"
] @keyword.storage.type

[
  "const"
] @keyword.storage.modifier

[
  "defer"
  "goto"
  "go"
] @function.macro

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
] @constant.numeric.integer

[
  (float_literal)
  (imaginary_literal)
] @constant.numeric.float

[
  (true)
  (false)
] @constant.builtin.boolean

[
  (nil)
  (iota)
] @constant.builtin

; Comments

(comment) @comment

; Doc Comments
(source_file
  .
  (comment)+ @comment.block.documentation)

(source_file
  (comment)+ @comment.block.documentation
  .
  (const_declaration))

(source_file
  (comment)+ @comment.block.documentation
  .
  (function_declaration))

(source_file
  (comment)+ @comment.block.documentation
  .
  (type_declaration))

(source_file
  (comment)+ @comment.block.documentation
  .
  (var_declaration))
