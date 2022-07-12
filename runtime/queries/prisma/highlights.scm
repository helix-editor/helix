(string) @string

(enumeral) @constant
(number) @constant.numeric

(variable) @variable
(column_type) @type

(arguments) @variable.other.member
(model_declaration (identifier) @type)

[
 "datasource"
 "enum"
 "generator"
 "model"
 "type"
] @keyword

[
 (comment)
 (developer_comment)
] @comment

[
 (attribute)
 (block_attribute_declaration)
 (call_expression)
] @function.builtin

[
 (true)
 (false)
 (null)
] @constant.builtin.boolean

[
 "("
 ")"
 "["
 "]"
 "{"
 "}"
] @punctuation.bracket

[
 ":" 
 ","
] @punctuation.delimiter

[
 "="
 "@"
 "@@"
 (binary_expression)
] @operator