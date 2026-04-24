; Comments

(comment) @comment.inside
(comment)+ @comment.around

; Functions

(function_definition body: (_) @function.inside) @function.around

; Parameters

(parameters
  ((_) @parameter.inside . (comma)? @parameter.around) @parameter.around)

(arguments
  ((_) @parameter.inside . (comma)? @parameter.around) @parameter.around)
