; Functions/constructors/predicates in Domain
(constructor_decl) @function.around

(function_decl) @function.around

(predicate_decl) @function.around

; Parameters
(parameter_list
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

; Arguments
(argument_list
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

; Comments
(comment) @comment.inside
(comment) @comment.around
