; Specify how to navigate around logical blocks in code

(assert_parameters
  ((_) @parameter.inside . ","? @parameter.around)) @parameter.around

(recipe
  (recipe_body) @function.inside) @function.around

(recipe_parameters
  ((_) @parameter.inside . ","? @parameter.around)) @parameter.around

(recipe_dependency
  (_) @parameter.inside) @parameter.around

(function_call
  (function_parameters
    ((_) @parameter.inside . ","? @parameter.around)) @parameter.around) @function.around

(comment) @comment.around
