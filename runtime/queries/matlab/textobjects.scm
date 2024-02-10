(arguments ((_) @parameter.inside . ","? @parameter.around) @parameter.around)
(function_arguments ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(lambda expression: (_) @function.inside) @function.around
(function_definition (block) @function.inside) @function.around

(class_definition) @class.inside @class.around

(comment) @comment.inside @comment.around
