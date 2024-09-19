(func_definition
  body: (_) @function.inside) @function.around

(struct_declaration
  body: (_) @class.inside) @class.around

(enum_declaration
  body: (_) @class.inside) @class.around

(fn_parameter_list 
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(line_comment) @comment.inside
(line_comment)+ @comment.around

(doc_comment) @comment.inside
(doc_comment)+ @comment.outside
