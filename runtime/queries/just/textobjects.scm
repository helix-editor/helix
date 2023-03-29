(body) @function.inside
(recipe) @function.outer
(expression 
    if:(expression) @function.inner 
) 
(expression 
    else:(expression) @function.inner
) 
(interpolation (expression) @function.inner) @function.outer
(settinglist (stringlist) @function.inner) @function.outer

(call (NAME) @class.inner) @class.outer
(dependency (NAME) @class.inner) @class.outer
(depcall (NAME) @class.inner)

(dependency) @parameter.outer
(depcall) @parameter.inner
(depcall (expression) @parameter.inner) 

(stringlist 
    (string) @parameter.inner
    . ","? @_end
    (#make-range! "parameter.outer" @parameter.inner @_end)
)
(parameters 
    [(parameter) 
    (variadic_parameters)] @parameter.inner
    . " "? @_end
    (#make-range! "parameter.outer" @parameter.inner @_end)
)

(expression 
    (condition) @function.inner
) @functio.outer
(expression 
    if:(expression) @function.inner 
)
(expression 
    else:(expression) @function.inner
)

(item [(alias) (assignment) (export) (setting)]) @class.outer
(recipeheader) @class.outer
(line) @class.outer

(comment) @comment.outer
