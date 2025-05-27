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
(lambda (name) @function)

; Function definitions

(function_definition 
  name: (name) @function
  parameters: (parameters) @variable.parameter )
(constructor_definition "_init" @function)
(lambda (parameters) @variable.parameter)


;; Literals
(comment) @comment
(string) @string

(type) @type
(expression_statement (array (identifier) @type))
(binary_operator (identifier) @type)
(enum_definition (name) @type.enum)
(enumerator (identifier) @type.enum.variant)
[
  (null)
  (underscore)
] @type.builtin


(variable_statement (identifier) @variable)
(attribute 
  (identifier) 
  (identifier) @variable.other.member)
(attribute 
  (identifier) @type.builtin
  (#match? @type.builtin "^(AABB|Array|Basis|bool|Callable|Color|Dictionary|float|int|NodePath|Object|Packed(Byte|Color|String)Array|PackedFloat(32|64)Array|PackedInt(32|64)Array|PackedVector(2|3)Array|Plane|Projection|Quaternion|Rect2([i]{0,1})|RID|Signal|String|StringName|Transform(2|3)D|Variant|Vector(2|3|4)([i]{0,1}))$"))

[
  (string_name)
  (node_path)
  (get_node)
] @label
(signal_statement (name) @label)

(const_statement (name) @constant)
(integer) @constant.numeric.integer
(float) @constant.numeric.float
(escape_sequence) @constant.character.escape
[
  (true)
  (false)
] @constant.builtin.boolean

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
  ":="
  ":"
] @operator

(annotation (identifier) @keyword.storage.modifier)

[
  "if"
  "else"
  "elif"
  "match"
  "when"
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
  "await"
] @keyword

