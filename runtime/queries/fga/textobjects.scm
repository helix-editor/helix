(condition_declaration
  body: (_) @function.inside) @function.around

(param 
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(comment) @comment.inside

(comment)+ @comment.around
