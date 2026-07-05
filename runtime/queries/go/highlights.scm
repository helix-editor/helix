
; Identifiers

(field_identifier) @variable.other.member

(identifier) @variable

(package_identifier) @namespace

(const_spec
  name: (identifier) @constant)

(keyed_element . (literal_element (identifier) @variable.other.member))
(field_declaration
  name: (field_identifier) @variable.other.member)

(parameter_declaration (identifier) @variable.parameter)
(variadic_parameter_declaration (identifier) @variable.parameter)

(label_name) @label

(const_spec
  name: (identifier) @constant)

; Function calls

(call_expression
  function: (identifier) @function)

((call_expression
  function: (identifier) @function.public)
  (#match? @function.public "^[A-Z]"))

(call_expression
  function: (selector_expression
    field: (field_identifier) @function.method))

((call_expression
  function: (selector_expression
    field: (field_identifier) @function.method.public))
  (#match? @function.method.public "^[A-Z]"))

(call_expression
  function: (identifier) @function.builtin
  (#match? @function.builtin "^(append|cap|close|complex|copy|delete|imag|len|make|new|panic|print|println|real|recover|min|max|clear)$"))

; Types

(type_identifier) @type

(type_parameter_list
  (type_parameter_declaration
    name: (identifier) @type.parameter))

((type_identifier) @type.builtin
  (#match? @type.builtin "^(any|bool|byte|comparable|complex128|complex64|error|float32|float64|int|int16|int32|int64|int8|rune|string|uint|uint16|uint32|uint64|uint8|uintptr)$"))

; Type definition names: `type Foo struct{}`, `type Bar = Baz`.
(type_spec
  name: (type_identifier) @type.definition)
(type_alias
  name: (type_identifier) @type.definition)

; Function definitions

(function_declaration
  name: (identifier) @function)

((function_declaration
  name: (identifier) @function.public)
  (#match? @function.public "^[A-Z]"))

(method_declaration
  name: (field_identifier) @function.method)

((method_declaration
  name: (field_identifier) @function.method.public)
  (#match? @function.method.public "^[A-Z]"))

(method_elem
  name: (field_identifier) @function.method)

((method_elem
  name: (field_identifier) @function.method.public)
  (#match? @function.method.public "^[A-Z]"))

; Blank identifier `_` (Go's discard) — dim as unused.
; It parses as (blank_identifier) in imports and as (identifier) elsewhere
; (`_ = x`, `a, _ := f()`, `for _, v := range`).
(blank_identifier) @comment.unused
((identifier) @comment.unused
 (#eq? @comment.unused "_"))

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
  "&^"
  "&^="
  "~"
] @operator

; Keywords

[
  "default"
  "type"
] @keyword

[
  "defer"
  "go"
  "goto"
] @keyword.control

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
] @string

(rune_literal) @constant.character

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
  (comment) @comment.block.documentation . (comment)* . [
    (package_clause) ; `package`
    (type_declaration) ; `type`
    (function_declaration) ; `func`
    (method_declaration) ; `func`
    (var_declaration) ; `var`
    (const_declaration) ; `const`
    ; var (
    ; 	A = 1
    ; 	B = 2
    ; )
    (var_spec)
    ; const (
    ; 	A = 1
    ; 	B = 2
    ; )
    (const_spec)
  ])
