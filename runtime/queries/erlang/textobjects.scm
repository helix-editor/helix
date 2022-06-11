(function_clause
  pattern: (arguments (_)? @parameter.inside)
  body: (_) @function.inside) @function.around

(anonymous_function
  (stab_clause body: (_) @function.inside)) @function.around

(comment (comment_content) @comment.inside) @comment.around
