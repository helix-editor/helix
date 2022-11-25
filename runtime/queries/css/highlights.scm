(comment) @comment

[
 (tag_name)
 (nesting_selector)
 (universal_selector)
] @tag

[
 "~"
 ">"
 "+"
 "-"
 "*"
 "/"
 "="
 "^="
 "|="
 "~="
 "$="
 "*="
] @operator

[
 "and"
 "not"
 "only"
 "or"
] @keyword.operator

((property_name) @variable
 (#match? @variable "^--"))
((plain_value) @variable
 (#match? @variable "^--"))

(attribute_name) @attribute
(class_name) @label
(feature_name) @variable.other.member
(function_name) @function
(id_name) @label
(namespace_name) @namespace
(property_name) @variable.other.member

[
 "@charset"
 "@import"
 "@keyframes"
 "@media"
 "@namespace"
 "@supports"
 (at_keyword)
 (from)
 (important)
 (to)
] @keyword

[
 "#"
 "."
] @punctuation

(string_value) @string
((color_value) "#") @string.special
(color_value) @string.special

(integer_value) @constant.numeric.integer
(float_value) @constant.numeric.float

[
 ")"
 "("
 "["
 "]"
 "{"
 "}"
] @punctuation.bracket

[
 ","
 ";"
 ":"
 "::"
] @punctuation.delimiter

(plain_value) @constant
