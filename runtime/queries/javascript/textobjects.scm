(function_declaration
  body: (statement_block)) @function.around

(function
  body: (statement_block)) @function.around

(export_statement
  (function_declaration) @function.around) @function.around.start

(arrow_function
  body: (statement_block)) @function.around

(method_definition
  body: (statement_block)) @function.around

(class_declaration
  body: (class_body) @class.inside) @class.around

(export_statement
  (class_declaration) @class.around) @class.around.start

(export_statement
  (class) @class.around) @class.around.start

(_ (statement_block) @block.inside) @block.around

(formal_parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(comment) @comment.inside

(comment)+ @comment.around
