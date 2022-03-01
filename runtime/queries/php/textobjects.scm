(class_declaration
  body: (_) @class.inside) @class.around

(interface_declaration
  body: (_) @class.inside) @class.around

(trait_declaration
  body: (_) @class.inside) @class.around

(enum_declaration
  body: (_) @class.inside) @class.around

(function_definition
  body: (_) @function.inside) @function.around

(method_declaration
  body: (_) @function.inside) @function.around

(arrow_function 
  body: (_) @function.inside) @function.around
  
(anonymous_function_creation_expression
  body: (_) @function.inside) @function.around
  
(formal_parameters
  [
    (simple_parameter)
    (variadic_parameter)
    (property_promotion_parameter)
  ] @parameter.inside)

(comment) @comment.inside

(comment)+ @comment.around
