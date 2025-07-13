(class_definition
  (class_body) @class.inside) @class.around

(call_expression
  (call_body
    (_) @function.inside) @function.around)

(method_definition
  body: (_) @function.inside) @function.around

(parameter_list
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(comment) @comment.inside

(comment)+ @comment.around
