(function_declaration
  body: (_) @function.inside) @function.around

(struct_union_type
  "{" @class.inside (_) @class.inside "}" @class.inside) @class.around

(enum_type
  "{" @class.inside (_) @class.inside "}" @class.inside) @class.around

(function_type
  ((parameter) @parameter.inside . ","? @parameter.around) @parameter.around)

(argument_list
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(comment) @comment.inside
(comment)+ @comment.around
