(class_declaration
  body: (_) @class.inside) @class.around

(protocol_declaration
  body: (_) @class.inside) @class.around

(function_declaration
  body: (_) @function.inside) @function.around

(parameter
  (_) @parameter.inside) @parameter.around

(lambda_parameter
  (_) @parameter.inside) @parameter.around

[
  (comment)
  (multiline_comment)
] @comment.inside

(comment)+ @comment.around

(multiline_comment) @comment.around
