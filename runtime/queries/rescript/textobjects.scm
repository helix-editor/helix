; Classes (modules)
;------------------

(module_declaration definition: ((_) @class.inside)) @class.around

; Functions
;----------

(function body: (_) @function.inside) @function.around

; Comments
;---------

(comment) @comment.inside

(comment)+ @comment.around
