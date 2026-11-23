(function_declaration
  body: (_) @function.inside) @function.around

(method_declaration
  body: (_) @function.inside) @function.around

(lambda
  body: (_) @function.inside) @function.around

(tree_declaration
  body: (_) @class.inside) @class.around

(trait_declaration
  body: (_) @class.inside) @class.around

(kind_declaration
  body: (_) @class.inside) @class.around

(parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(lambda_parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(type_arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(comment) @comment.inside

(comment)+ @comment.around

(list
  ((_) @entry.inside . ","? @entry.around) @entry.around)

(map
  ((pair) @entry.inside . ","? @entry.around) @entry.around)

(kind_body
  ((kind_case) @entry.inside . ","? @entry.around) @entry.around)

(field_declaration) @entry.around
