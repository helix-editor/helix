(identifier) @variable
(type_identifier) @type
(comment) @comment
(int_literal) @number
(float_literal) @float
(string_literal) @string
(function_identifier) @function
[
(image_macro)
(children_macro)
(radial_grad_macro)
(linear_grad_macro)
] @function.macro
(call_expression
  function: (identifier) @function.call)
(call_expression
  function: (field_expression
    field: (identifier) @function.call))
(vis) @include
(units) @type
(array_literal 
  (identifier) @type)
(transition_statement state: (identifier) @field)
(state_expression state: (identifier) @field)
(struct_block_definition 
  (identifier) @field)

; (state_identifier) @field

[
"in"
"for"
] @repeat

"@" @keyword

[
"import" 
"from"
] @include

[
"if"
"else"
] @conditional

[
"root"
"parent"
"duration"
"easing"
] @variable.builtin

[
"true"
"false"
] @boolean


[
"struct"
"property"
"callback"
"in"
"animate"
"states"
"when"
"out"
"transitions"
"global"
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
"red"
"start"
"yellow"
"white"
"gray"
] @constant.builtin


; Punctuation
[
","
"."
";"
":"
] @punctuation.delimiter

; Brackets
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
 "=>"
 ] @operator

(ternary_expression [":" "?"] @conditional)