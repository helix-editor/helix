(function_definition 
  (identifier) (_) @function.inside) @function.around

(anonymous_function 
  (_) @function.inside) @function.around

(argument 
  (_) @parameter.inside)

[
  (comments)
  (pod_statement)
] @comment.inside

(comments)+ @comment.around

(pod_statement) @comment.around
