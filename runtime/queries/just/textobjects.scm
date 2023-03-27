(body) @block.inner
(body) @block.outer
(body) @function.inner
(recipe) @function.outer
(expression 
    if:(expression) @block.inner 
) 
(expression 
    else:(expression) @block.inner
) 
(interpolation (expression) @block.inner) @block.outer
(settinglist (stringlist) @block.inner) @block.outer

(call (NAME) @call.inner) @call.outer
(dependency (NAME) @call.inner) @call.outer
(depcall (NAME) @call.inner)

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
    (condition) @conditional.inner
) @conditional.outer
(expression 
    if:(expression) @conditional.inner 
)
(expression 
    else:(expression) @conditional.inner
)

(item [(alias) (assignment) (export) (setting)]) @statement.outer
(recipeheader) @statement.outer
(line) @statement.outer

(comment) @comment.outer
