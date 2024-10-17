[(comment) (single_line_comment)] @comment

"~" @operator
">" @operator
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"=" @operator
"^=" @operator
"|=" @operator
"~=" @operator
"$=" @operator
"*=" @operator

"in" @operator
"and" @operator
"or" @operator
"not" @operator
"only" @operator

"@apply" @constant.builtin
"@at-root" @constant.builtin
"@charset" @constant.builtin
"@debug" @constant.builtin
"@each" @keyword.control.repeat
"@else" @keyword.control.conditional
"@error" @constant.builtin
"@extend" @constant.builtin
"@for" @keyword.control.repeat
"@forward" @keyword.control.import
"@function" @function.method
"@if" @keyword.control.conditional
"@import" @keyword.control.import
"@include" @keyword.control.import
"@keyframes" @constant.builtin
"@media" @constant.builtin
"@mixin" @constant.builtin
"@namespace" @namespace
"@return" @keyword.control.return
"@supports" @constant.builtin
"@use" @keyword.control.import
"@warn" @constant.builtin
"@while" @keyword.control.repeat

((property_name) @variable
 (#match? @variable "^--"))
((plain_value) @variable
 (#match? @variable "^--"))

(tag_name) @tag
(universal_selector) @tag
(attribute_selector (plain_value) @string)
(nesting_selector) @variable.other.member
(pseudo_element_selector) @attribute
(pseudo_class_selector) @attribute

(identifier) @variable
(class_name) @variable
(id_name) @variable
(namespace_name) @variable
(property_name) @variable.other.member
(feature_name) @variable
(variable) @variable
(variable_name) @variable.other.member
(variable_value) @variable.other.member
(argument_name) @variable.parameter
(selectors) @variable.other.member

(attribute_name) @attribute

(function_name) @function

(to) @keyword
(from) @keyword
(important) @keyword

(string_value) @string
(color_value) @string.special

(integer_value) @constant.numeric.integer
(float_value) @constant.numeric.float
(unit) @type

"#" @punctuation.delimiter
"," @punctuation.delimiter
