(function_declaration) @function.around

(function_declaration
  body: (_) @function.inside)

(function_declaration
  (fndec_attrs
    (fndec_attr "@test"))) @test.around

(function_declaration
  (fndec_attrs
    (fndec_attr "@test"))
  body: (_) @test.inside)

(parameter) @parameter.around

(switch_case) @parameter.around

(comment) @comment.inside

(comment)+ @comment.around

(type_declaration) @class.around
