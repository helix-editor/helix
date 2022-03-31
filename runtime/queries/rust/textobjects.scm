(function_item
  body: (_) @function.inside) @function.around

(struct_item
  body: (_) @class.inside) @class.around

(enum_item
  body: (_) @class.inside) @class.around

(union_item
  body: (_) @class.inside) @class.around

(trait_item
  body: (_) @class.inside) @class.around

(impl_item
  body: (_) @class.inside) @class.around

(parameters
  (_) @parameter.inside)

(closure_parameters
  (_) @parameter.inside)

(arguments
  (_) @parameter.inside)

[
  (line_comment)
  (block_comment)
] @comment.inside

(line_comment)+ @comment.around

(block_comment) @comment.around
