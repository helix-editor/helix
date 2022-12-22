; Classes (modules)
;------------------

(module_declaration definition: ((_) @class.inside)) @class.around

; Blocks
;-------

(block (_) @function.inside) @function.around

; Functions
;----------

(function body: (_) @function.inside) @function.around

; Calls
;------

(call_expression arguments: ((_) @parameter.inside)) @parameter.around

; Comments
;---------

(comment) @comment.inside
(comment)+ @comment.around

; Parameters
;-----------

(function parameter: (_) @parameter.inside @parameter.around)

(formal_parameters
  ","
  . (_) @parameter.inside
  @parameter.around)
(formal_parameters
  . (_) @parameter.inside
  . ","?
  @parameter.around)

(arguments
  "," @_arguments_start
  . (_) @parameter.inside
  @parameter.around)
(arguments
  . (_) @parameter.inside
  . ","?
  @parameter.around)

(function_type_parameters
  ","
  . (_) @parameter.inside
  @parameter.around)
(function_type_parameters
  . (_) @parameter.inside
  . ","?
  @parameter.around)

(functor_parameters
  ","
  . (_) @parameter.inside
  @parameter.around)
(functor_parameters
  . (_) @parameter.inside
  . ","?
  @parameter.around)

(type_parameters
  ","
  . (_) @parameter.inside
  @parameter.around)
(type_parameters
  . (_) @parameter.inside
  . ","?
  @parameter.around)

(type_arguments
  ","
  . (_) @parameter.inside
  @parameter.around)
(type_arguments
  . (_) @parameter.inside
  . ","?
  @parameter.around)

(decorator_arguments
  ","
  . (_) @parameter.inside
  @parameter.around)
(decorator_arguments
  . (_) @parameter.inside
  . ","?
  @parameter.around)

(variant_parameters
  ","
  . (_) @parameter.inside
  @parameter.around)
(variant_parameters
  . (_) @parameter.inside
  . ","?
  @parameter.around)

(polyvar_parameters
  ","
  . (_) @parameter.inside
  @parameter.around)
(polyvar_parameters
  . (_) @parameter.inside
  . ","?
  @parameter.around)

