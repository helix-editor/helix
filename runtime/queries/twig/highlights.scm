(comment) @comment

(filter_identifier) @function.method
(function_identifier) @function.method
(test) @function.builtin
(variable) @variable
(string) @string
(interpolated_string) @string
(operator) @operator
(number) @constant.numeric.integer
(boolean) @constant.builtin.boolean
(null) @constant.builtin
(keyword) @keyword
(attribute) @attribute
(tag) @tag
(conditional) @keyword.control.conditional
(repeat) @keyword.control.repeat
(method) @function.method
(parameter) @variable.parameter

[
    "{{"
    "}}"
    "{{-"
    "-}}"
    "{{~"
    "~}}"
    "{%"
    "%}"
    "{%-"
    "-%}"
    "{%~"
    "~%}"
] @keyword

[
    ","
    "."
    "?"
    ":"
    "="
] @punctuation.delimiter

(interpolated_string [
    "#{" 
    "}"
] @punctuation.delimiter)

[
    "("
    ")"
    "["
    "]"
    "{"
] @punctuation.bracket

(hash [
    "}"
] @punctuation.bracket)

