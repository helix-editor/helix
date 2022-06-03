(function
  parameters: (function_parameters (function_parameter)? @parameter.inside)
  body: (function_body) @function.inside) @function.around

(anonymous_function
  body: (function_body) @function.inside) @function.around
