; Classes (modules)
;------------------

(module_declaration definition: ((_) @class.inner)) @class.outer

; Functions
;----------

(function body: (_) @function.inner) @function.outer

; Parameters
;-----------

(function parameter: (_) @parameter.inner @parameter.outer)
