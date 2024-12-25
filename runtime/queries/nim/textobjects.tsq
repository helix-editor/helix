(proc_declaration
  body: (_) @function.inside) @function.around
(func_declaration
  body: (_) @function.inside) @function.around
(iterator_declaration
  body: (_) @function.inside) @function.around
(converter_declaration
  body: (_) @function.inside) @function.around
(method_declaration
  body: (_) @function.inside) @function.around
(template_declaration
  body: (_) @function.inside) @function.around
(macro_declaration
  body: (_) @function.inside) @function.around

(type_declaration (_) @class.inside) @class.around

(parameter_declaration
  (symbol_declaration_list) @parameter.inside) @parameter.around

[
  (comment)
  (block_comment)
  (documentation_comment)
  (block_documentation_comment)
] @comment.inside

[
  (comment)+
  (block_comment)
  (documentation_comment)+
  (block_documentation_comment)+
] @comment.around
