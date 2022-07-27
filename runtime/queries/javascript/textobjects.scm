(function_declaration
  body: (statement_block)) @function.around

(function
  body: (statement_block)) @function.around

(export_statement
  declaration: (function_declaration) @function.around) 

(arrow_function
  body: (statement_block)) @function.around

(method_definition
  body: (statement_block)) @function.around

(generator_function_declaration
  body: (_) @function.inside) @function.around

(class_declaration
  body: (class_body) @class.inside) @class.around

(class
  (class_body) @class.inside) @class.around

(export_statement
  declaration: (class_declaration) @class.around) 

(formal_parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(comment) @comment.inside

(comment)+ @comment.around
