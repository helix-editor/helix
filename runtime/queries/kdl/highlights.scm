(comment) @comment
(single_line_comment) @comment

(node
    name: (identifier) @function)
(prop (identifier) @attribute)
(type) @type

(bare_identifier) @variable.other.member

(keyword) @keyword

(string) @string
(number) @constant.numeric
(boolean) @constant.builtin.boolean

"." @punctuation.delimiter

"=" @operator

"{" @punctuation.bracket
"}" @punctuation.bracket
