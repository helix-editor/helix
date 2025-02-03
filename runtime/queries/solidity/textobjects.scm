(function_definition
  body: (_) @function.inside) @function.around

(constructor_definition
  body: (_) @function.inside) @function.around

(fallback_receive_definition
  body: (_) @function.inside) @function.around

(yul_function_definition
  (yul_block) @function.inside) @function.around

(function_definition 
  ((parameter) @parameter.inside . ","? @parameter.around) @parameter.around)

(constructor_definition 
  ((parameter) @parameter.inside . ","? @parameter.around) @parameter.around)

(return_type_definition 
  ((parameter) @entry.inside . ","? @entry.around) @entry.around)

(modifier_definition 
  ((parameter) @parameter.inside . ","? @parameter.around) @parameter.around)

(event_definition 
  ((event_parameter) @parameter.inside . ","? @parameter.around) @parameter.around)

(error_declaration 
  ((error_parameter) @parameter.inside . ","? @parameter.around) @parameter.around)

(call_argument
  ((call_struct_argument) @entry.inside . ","? @entry.around) @entry.around)

(call_expression
  ((call_argument) @parameter.inside . ","? @parameter.around) @parameter.around)

(variable_declaration_tuple
  ((variable_declaration) @entry.inside . ","? @entry.around) @entry.around)

(emit_statement
  ((call_argument) @parameter.inside . ","? @parameter.around) @parameter.around)

(revert_arguments
  ((call_argument) @parameter.inside . ","? @parameter.around) @parameter.around)

(struct_declaration
  body: (_) @class.inside) @class.around

(enum_declaration
  body: (_) @class.inside) @class.around

(comment) @comment.inside

(comment)+ @comment.around

