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
] @tag

"end" @keyword.return

(variable_name) @variable
(filter_name) @function.macro
(filter_argument) @variable.parameter
(tag_name) @function
(keyword) @keyword
(operator) @operator
(keyword_operator) @keyword.directive
(number) @constant.numeric
(boolean) @constant.builtin.boolean
(string) @string
