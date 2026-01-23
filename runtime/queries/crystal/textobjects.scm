(class_def
  name: (_)
  (_) @class.inside) @class.around

(struct_def
  name: (_)
  (_) @class.inside) @class.around

(module_def
  name: (_)
  (_) @class.inside) @class.around

(lib_def
  name: (_)
  (_) @class.inside) @class.around

(enum_def
  name: (_)
  (_) @class.inside) @class.around

(block
  params: (_) @parameter.inside) @parameter.around

(method_def
  params: (_) @parameter.inside) @parameter.around

(method_def
  name: (_)
  (_) @function.inside) @function.around

(block
  (_) @function.inside) @function.around

(comment) @comment.inside
(comment)+ @comment.around

(array
  (_) @entry.around)
