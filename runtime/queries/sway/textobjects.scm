(function_item
  body: (_) @function.inside) @function.around(closure_expression body: (_) @function.inside) @function.around

(struct_item
  body: (_) @class.inside) @class.around

(enum_item
  body: (_) @class.inside) @class.around

(trait_item
  body: (_) @class.inside) @class.around

(impl_item
  body: (_) @class.inside) @class.around

(parameters 
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(type_parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(type_arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(closure_parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

[
  (line_comment)
  (block_comment)
] @comment.inside

(line_comment)+ @comment.around

(block_comment) @comment.around

(; #[test]
 (attribute_item
   (attribute
     (identifier) @_test_attribute))
 ; allow other attributes like #[should_panic] and comments
 [
   (attribute_item)
   (line_comment)
 ]*
 ; the test function
 (function_item
   body: (_) @test.inside) @test.around
 (#eq? @_test_attribute "test"))
