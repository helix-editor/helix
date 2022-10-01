
(user_type_identifier) @type

(var_identifier) @variable

(state_identifier) @variable.other.member

(var_identifier
  (post_identifier) @variable)

(function_identifier) @function

(reference_identifier) @keyword.storage.modifier.ref
(visibility_modifier) @keyword.storage.modifier

(comment) @comment

(string) @string
(int_number) @constant.numeric
(unit_type) @type.builtin

[
"struct"
"property"
"callback"
"import"
"from"
"root"
"parent"
"this"
"for"
"in"
"if"
"else if"
"else"
"animate"
"states"
"when"
"in"
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
 "true"
 "false"
 ] @constant.builtin

"@" @keyword

; ; Punctuation
[
  ","
  "."
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

[
"angle"
"bool"
"brush"
; "color" // This causes problems
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

(ternary_expression [":" "?"] @keyword.control.conditional)