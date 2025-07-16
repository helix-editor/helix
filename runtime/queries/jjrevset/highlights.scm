(at_op) @variable.builtin

[
  "::" ".."
  (negate_op)
  (union_op) (intersection_op) (difference_op)
] @operator

["(" ")"] @punctuation.bracket
"," @punctuation.delimiter
[(raw_string_literal) (string_literal)] @string

(function ((strict_identifier) @function))
(function (function_arguments (keyword_argument (strict_identifier) @variable.parameter)))

(primary ((identifier) @variable))

(string_pattern (strict_identifier) @keyword)
