
(identifier) @variable
[
  (type_identifier) 
  (units)
]@type

(array_literal 
  (identifier) @type)

(function_identifier) @function
[
  (image_macro)
  (children_macro)
  (radial_grad_macro)
  (linear_grad_macro)
] @function.macro

(call_expression
  function: (identifier) @function)
(call_expression
  function: (field_expression
    field: (identifier) @function))

(vis) @keyword.control.import

(transition_statement state: (identifier) @variable.other.member)
(state_expression state: (identifier) @variable.other.member)
(struct_block_definition field: (identifier) @variable.other.member)
(assign_property (identifier) @attribute)

(comment) @comment

(string_literal) @string
(int_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float

[
  "in"
  "in-out"
  "for"
] @keyword.control.repeat

[
  "import"
  "export"
  "from"
] @keyword.control.import

[
  "if"
  "else"
  "when"
] @keyword.control.conditional

[
  "struct"
  "property"
] @keyword.storage.type

[
  "global"
] @keyword.storage.modifier


[
  "root"
  "parent"
  "duration"
  "easing"
] @variable.builtin


[
  "callback"
  "animate"
  "states"
  "out"
  "transitions"
  "component"
  "inherits"
] @keyword

[
  "black"
  "transparent"
  "blue"
  "ease"
  "ease_in"
  "ease-in"
  "ease_in_out"
  "ease-in-out"
  "ease_out"
  "ease-out"
  "end"
  "green"
  "red"
  "start"
  "yellow"
  "white"
  "gray"
 ] @constant.builtin

[
  "true"
  "false"
] @constant.builtin.boolean

"@" @keyword

; ; Punctuation
[
  ","
  "."
  ";"
  ":"
] @punctuation.delimiter

; ; Brackets
[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

(define_property ["<" ">"] @punctuation.bracket)

[
  "angle"
  "bool"
  "brush"
  "color" 
  "duration"
  "easing"
  "float"
  "image"
  "int"
  "length"
  "percent"
  "physical-length"
  "physical_length"
  "string"
] @type.builtin

[
 ":="
 "<=>"
 "!"
 "-"
 "+"
 "*"
 "/"
 "&&"
 "||"
 ">"
 "<"
 ">="
 "<="
 "="
 ":"
 "+="
 "-="
 "*="
 "/="
 "?"
 "=>" ] @operator

(ternary_expression [":" "?"] @keyword.control.conditional)