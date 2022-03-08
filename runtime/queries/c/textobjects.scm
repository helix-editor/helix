(function_definition
  body: (_) @function.inside) @function.around

(struct_specifier
  body: (_) @class.inside) @class.around

(enum_specifier
  body: (_) @class.inside) @class.around

(union_specifier
  body: (_) @class.inside) @class.around

(parameter_declaration) @parameter.inside

(comment) @comment.inside

(comment)+ @comment.around
