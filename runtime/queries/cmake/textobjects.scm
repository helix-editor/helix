[
  (macro_def)
  (function_def)
] @function.around

(argument) @parameter.inside

[
  (bracket_comment)
  (line_comment)
] @comment.inside

(line_comment)+ @comment.around

(bracket_comment) @comment.around