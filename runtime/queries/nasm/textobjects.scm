(preproc_multiline_macro
  body: (body) @function.inside) @function.around
(struc_declaration
  body: (struc_declaration_body) @class.inside) @class.around
(struc_instance
  body: (struc_instance_body) @class.inside) @class.around

(preproc_function_def_parameters
  (word) @parameter.inside)
(call_syntax_arguments
  (_) @parameter.inside)
(operand) @parameter.inside

(comment) @comment.inside
(comment)+ @comment.around
