(function_declaration
  body: (block)? @function.inside) @function.around

((function_declaration
   name: (identifier) @_name
   body: (block)? @test.inside) @test.around
 (#match? @_name "^test"))

(function_literal
  body: (block)? @function.inside) @function.around

(parameter_list
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(call_expression
  (argument_list
    ((_) @parameter.inside) @parameter.around))

(struct_declaration
    (struct_field_declaration) @class.inside) @class.around

(struct_field_declaration
  ((_) @parameter.inside) @parameter.around)

[(line_comment) (block_comment)] @comment.inside
[(line_comment)+ (block_comment)+] @comment.around

