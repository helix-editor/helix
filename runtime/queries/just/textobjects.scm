(body) @function.inside
(recipe) @function.around
(expression 
    if:(expression) @function.inside 
) 
(expression 
    else:(expression) @function.inside
) 
(interpolation (expression) @function.inside) @function.around
(settinglist (stringlist) @function.inside) @function.around

(call (NAME) @class.inside) @class.around
(dependency (NAME) @class.inside) @class.around
(depcall (NAME) @class.inside)

(dependency) @parameter.around
(depcall) @parameter.inside
(depcall (expression) @parameter.inside) 

(stringlist 
    (string) @parameter.inside
    . ","? @_end
    (#make-range! "parameter.around" @parameter.inside @_end)
)
(parameters 
    [(parameter) 
    (variadic_parameters)] @parameter.inside
    . " "? @_end
    (#make-range! "parameter.around" @parameter.inside @_end)
)

(expression 
    (condition) @function.inside
) @functio.around
(expression 
    if:(expression) @function.inside 
)
(expression 
    else:(expression) @function.inside
)

(item [(alias) (assignment) (export) (setting)]) @class.around
(recipeheader) @class.around
(line) @class.around

(comment) @comment.around
