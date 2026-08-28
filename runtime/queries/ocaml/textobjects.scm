; Value/function definitions
(value_definition) @function.around
(value_definition
  (let_binding
    body: (_) @function.inside))

(fun_expression
  body: (_) @function.inside) @function.around

(function_expression) @function.around

(method_definition
  body: (_) @function.inside) @function.around

; Types and modules
(type_definition) @class.around
(type_definition
  (type_binding
    body: (_) @class.inside))

(module_definition) @class.around
(module_definition
  (module_binding
    body: (_) @class.inside))

; Parameters
(parameter) @parameter.around
(parameter
  pattern: (_) @parameter.inside)

(comment) @comment.inside
(comment)+ @comment.around
