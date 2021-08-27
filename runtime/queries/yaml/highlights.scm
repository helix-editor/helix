(block_mapping_pair key: (_) @field)
(flow_mapping (_ key: (_) @field))
(boolean_scalar) @boolean
(null_scalar) @constant.builtin
(double_quote_scalar) @string
(single_quote_scalar) @string
(escape_sequence) @string.escape
(integer_scalar) @number
(float_scalar) @number
(comment) @comment
(anchor_name) @type
(alias_name) @type
(tag) @type
(yaml_directive) @keyword
(ERROR) @error

[
","
"-"
":"
">"
"?"
"|"
] @punctuation.delimiter

[
"["
"]"
"{"
"}"
] @punctuation.bracket

["*" "&"] @punctuation.special
