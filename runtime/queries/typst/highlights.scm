([(show_expression) (set_expression)] @constant)
(["#"] @punctuation)
(["+"] @operator)
(["(" ")" "{" "}" "[" "]"] @punctuation.brackets)
([(line_comment) (block_comment)] @comment)
((identifier) @function)
((string_literal) @string)
((asssigned_argument) @type)