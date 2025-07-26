[
  (unpaired_comment)
  (paired_comment)
] @comment

[
  "{{"
  "}}"
  "{%"
  "%}"
  (end_paired_statement)
] @punctuation.bracket

[
 (tag_name) 
] @function

(variable_name) @variable
(filter_name) @function
(filter_argument) @variable.parameter
(keyword) @keyword
(operator) @operator
(keyword_operator) @keyword.operator
(number) @constant.numeric
(boolean) @constant.builtin.boolean
(string) @string
