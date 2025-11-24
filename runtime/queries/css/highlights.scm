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

(attribute_selector (plain_value) @string)

(property_name) @variable.other.member
(plain_value) @constant

((property_name) @variable
  (#match? @variable "^--"))
((plain_value) @variable
  (#match? @variable "^--"))

(class_name) @label
(feature_name) @variable.other.member
(function_name) @function
(id_name) @label
(namespace_name) @namespace

(attribute_name) @attribute
(pseudo_element_selector (tag_name) @attribute)
(pseudo_class_selector (class_name) @attribute)

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
  (keyword_query)
  (keyframes_name)
  (unit)
] @keyword

; @apply something;
(at_rule
  . (at_keyword) @keyword
  . (keyword_query) @constant
  (#eq? @keyword "@apply"))

[
  "#"
  "."
] @punctuation

(string_value) @string
(color_value "#" @string.special)
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
