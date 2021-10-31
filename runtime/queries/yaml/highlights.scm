(block_mapping_pair key: (_) @variable.other.member)
(flow_mapping (_ key: (_) @variable.other.member))
(boolean_scalar) @constant.builtin.boolean
(null_scalar) @constant.builtin
(double_quote_scalar) @string
(single_quote_scalar) @string
(escape_sequence) @constant.character.escape
(integer_scalar) @constant.numeric.integer
(float_scalar) @constant.numeric.float
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
