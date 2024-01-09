; Function queries

(function_definition
  body: (_) @function.inside) @function.around

; Does not match block lambdas or Scala 3 braceless lambdas
(lambda_expression
  (_) @function.inside) @function.around


; Class queries

(object_definition
  body: (_)? @class.inside) @class.around

(class_definition
  body: (_)? @class.inside) @class.around

(trait_definition
  body: (_)? @class.inside) @class.around

(type_definition) @class.around

(enum_case_definitions) @class.around

(enum_definition
  body: (_)? @class.inside) @class.around


; Parameter queries

(parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(parameter_types
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(bindings
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

; Does not match context bounds or higher-kinded types
(type_parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(type_arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)


; Comment queries

(comment) @comment.inside
(comment) @comment.around ; Does not match consecutive block comments


; Test queries
; Not supported
