(class_declaration
  body: (_) @class.inside) @class.around

(method_declaration
  body: (_) @function.inside) @function.around

(parameter (_) @parameter.inside) @parameter.around

(comment)+ @comment.around
