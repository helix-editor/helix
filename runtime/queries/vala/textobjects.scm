(method_declaration
  (block) @function.inside) @function.around

(creation_method_declaration
  (block) @function.inside) @function.around

(method_declaration
  ((parameter) @parameter.inside . ","? @parameter.around) @parameter.around)

[
  (class_declaration)
  (struct_declaration)
  (interface_declaration)
] @class.around

(type_arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(creation_method_declaration
  ((parameter) @parameter.inside . ","? @parameter.around) @parameter.around)

(method_call_expression
  ((argument) @parameter.inside . ","? @parameter.around) @parameter.around)

(comment) @comment.inside

(comment)+ @comment.around
