(function_declaration
  body: (_) @function.inside) @function.around

(struct_declaration) @class.around

[
  (struct_member)
  (parameter)
  (variable_declaration)
] @parameter.around

(comment) @comment.inside

(comment)+ @comment.around
