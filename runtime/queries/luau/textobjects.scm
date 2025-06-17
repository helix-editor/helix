(fn_stmt
  body: (_)? @function.inside) @function.around

(local_fn_stmt
  body: (_)? @function.inside) @function.around

(anon_fn
  body: (_)? @function.inside) @function.around

(param
  ((name) @parameter.inside . ","? @parameter.around) @parameter.around)

(arglist
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(comment) @comment.inside

(comment)+ @comment.around
