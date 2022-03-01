(function_definition
  body: (block)? @function.inside) @function.around

(class_definition
  body: (block)? @class.inside) @class.around

(parameters
  (_) @parameter.inside)
  
(lambda_parameters
  (_) @parameter.inside)

(argument_list
  (_) @parameter.inside)

(comment) @comment.inside

(comment)+ @comment.around
