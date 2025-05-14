(function_definition
  body: (_) @function.inside) @function.around

(alias_declaration
  body: (_) @function.inside) @function.around

(enum_struct_method
  body: (_) @function.inside) @function.around

(methodmap_method
  body: (_) @function.inside) @function.around

(methodmap_method_constructor
  body: (_) @function.inside) @function.around

(methodmap_method_destructor
  body: (_) @function.inside) @function.around

(methodmap_property_method
  body: (_) @function.inside) @function.around

(enum_struct) @class.around

(methodmap) @class.around

(parameter_declarations 
  ((parameter_declaration) @parameter.inside . ","? @parameter.around) @parameter.around)

(comment) @comment.around
