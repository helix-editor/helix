(function_declaration
  body: (_) @function.inside) @function.around

[
  (struct_declaration)
  (enum_declaration)
  (impl_declaration)
] @class.around

(parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(comment) @comment.inside
(comment)+ @comment.around

(function_declaration) @test.around
