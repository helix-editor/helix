(comment) @comment.inside
(comment)+ @comment.around

(formals
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(function_expression
  body: (_) @function.inside) @function.around

