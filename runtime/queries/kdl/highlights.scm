(single_line_comment) @comment
(multi_line_comment) @comment

(node
    (identifier) @function)
(prop (identifier) @attribute)
(type) @type

(keyword) @keyword

(string) @string
(number) @constant.numeric
(boolean) @constant.builtin.boolean

"." @punctuation.delimiter

"=" @operator

"{" @punctuation.bracket
"}" @punctuation.bracket
