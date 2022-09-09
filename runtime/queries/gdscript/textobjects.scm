
(class_definition
  (body) @class.inside) @class.around

(function_definition
  (body) @function.inside) @function.around

(parameters
  (typed_parameter 
    (identifier) @parameter.inside
  ) @parameter.around)

(comment) @comment.inside
(comment)+ @comment.around
