(comment) @comment

(match
  _ ("{" (_) @variable.parameter "}")?) @constant.builtin

(assignment
  _ ("{" (_) @variable.parameter "}")?) @function.builtin

(var_sub
  _ "{" (_) @variable.parameter "}" ) @function.macro

[
  (match_op)
  (assignment_op)
] @operator

(value) @string
(linebreak) @constant.builtin

[
  ","
] @punctuation.delimiter

[
  "{"
  "}"
] @punctuation.bracket
