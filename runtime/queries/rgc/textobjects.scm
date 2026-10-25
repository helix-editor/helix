; Text objects for RGC: functions, classes, parameters, comments.

(function_definition
  body: (_) @function.inside) @function.around

(class_definition
  body: (_) @class.inside) @class.around

(enum_definition
  body: (_) @class.inside) @class.around

(interface_definition
  body: (_) @class.inside) @class.around

(parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(comment) @comment.inside

(comment)+ @comment.around
