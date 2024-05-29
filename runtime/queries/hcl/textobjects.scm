(comment) @comment.inside
(comment)+ @comment.around

(function_arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

