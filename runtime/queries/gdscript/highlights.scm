; Identifier naming conventions

(
  (identifier) @constant 
  (#match? @constant "^[A-Z][A-Z\\d_]+$"))

; class
(class_name_statement (name) @type)
(class_definition (name) @type)


; Function calls

(attribute_call (identifier) @function)
(base_call (identifier) @function)
(call (identifier) @function)

; Function definitions

(function_definition (name) @function)
(constructor_definition "_init" @function)


;; Literals
(comment) @comment
(string) @string

(type) @type
(expression_statement (array (identifier) @type))
(binary_operator (identifier) @type)

(variable_statement (identifier) @variable)
(get_node) @label

(const_statement (name) @constant)
(integer) @constant.numeric.integer
(float) @constant.numeric.float
(escape_sequence) @constant.character.escape
[
  (true)
  (false)
] @constant.builtin.boolean
(null) @constant.builtin

[
  "+"
  "-"
  "*"
  "/"
  "%"
  "=="
  "!="
  ">"
  "<"
  ">="
  "<="
  "="
  "+="
  "-="
  "*="
  "/="
  "%="
  "&"
  "|"
  "^"
  "~"
  "<<"
  ">>"
] @operator

(annotation (identifier) @keyword.storage.modifier)

[
  "if"
  "else"
  "elif"
] @keyword.control.conditional

[
  "while"
  "for"
] @keyword.control.repeat

[
  "return"
  "pass"
  "break"
  "continue"
] @keyword.control.return

[
  "func"
] @keyword.control.function

[
  "export"
] @keyword.control.import

[
  "in"
  "is"
  "as"
  "match"
  "and"
  "or"
  "not"
] @keyword.operator

[
  "var"
  "class"
  "class_name"
  "enum"
] @keyword.storage.type


[
  (remote_keyword)
  (static_keyword)
  "const"
  "signal"
  "@"
] @keyword.storage.modifier

[
  "setget"
  "onready"
  "extends"
  "set"
  "get"
] @keyword

