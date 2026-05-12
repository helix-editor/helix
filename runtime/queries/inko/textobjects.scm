(class
  body: (_) @class.inside) @class.around

(trait
  body: (_) @class.inside) @class.around

(method
  body: (_) @function.inside) @function.around

(reopen_class
  body: (_) @class.inside) @class.around

(implement_trait
  body: (_) @class.inside) @class.around

(external_function
  body: (_) @function.inside) @function.around

(closure
  body: (_) @function.inside) @function.around

(arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(type_arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(line_comment) @comment.inside

(line_comment)+ @comment.around

(array (_) @entry.around)

(tuple (_) @entry.around)

(tuple_pattern (_) @entry.around)

(define_field (_) @entry.inside) @entry.around
