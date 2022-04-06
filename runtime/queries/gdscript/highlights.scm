; Identifier naming conventions

((identifier) @constant
 (#match? @constant "^[A-Z][A-Z_]*$"))

; Function calls

(attribute_call (identifier) @function)

(base_call (identifier) @function)

(call (identifier) @function)

; Function definitions

(function_definition (name) @function)

(constructor_definition "_init" @function)

;; Literals
(integer) @constant.numeric.integer
(float) @constant.numeric.float
(comment) @comment
(string) @string
(escape_sequence) @constant.character.escape
(identifier) @variable
(type) @type

;; Literals
[
  (true)
  (false)
  (null)
] @constant.builtin

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
  "and"
  "or"
  "not"
] @operator

[
  (static_keyword)
  (remote_keyword)
  (tool_statement)
  "var"
  "func"
  "setget"
  "in"
  "is"
  "as"
  "if"
  "else"
  "elif"
  "while"
  "for"
  "return"
  "break"
  "continue"
  "pass"
  "match"
  "class"
  "class_name"
  "enum"
  "signal"
  "onready"
  "export"
  "extends"
  "const"
] @keyword
