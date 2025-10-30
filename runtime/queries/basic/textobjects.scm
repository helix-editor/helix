(function_definition
  body: (_)? @function.inside) @function.around

(sub_definition
  body: (_)? @function.inside) @function.around

(for_statement
  body: (_)? @function.inside) @function.around

(comment) @comment.inside
(comment)+ @comment.around
