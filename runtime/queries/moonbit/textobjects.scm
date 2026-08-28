; Functions

((function_definition
  ("fn"
   (block_expression) @function.inside)
   (#offset! @function.inside 0 1 0 -1))
 @function.around)

((function_definition
  (external_source (_) @function.inside)
  .)
 @function.around)

(trait_method_declaration) @function.around

((impl_definition
  ("impl"
   (block_expression) @function.inside)
   (#offset! @function.inside 0 1 0 -1))
 @function.around)

; Tests

((test_definition
  ("test"
   (block_expression) @test.inside)
   (#offset! @test.inside 0 1 0 -1))
 @test.around)

; Classes (type-like definitions)

(struct_definition) @class.around
(enum_definition) @class.around
(trait_definition) @class.around
(type_definition) @class.around

; Parameters

((parameters
  .
  (parameter) @parameter.inside
  .
  ","? @_end)
  (#make-range! "parameter.around" @parameter.inside @_end))

((trait_method_declaration
  (trait_method_parameter) @parameter.inside
  .
  ","? @_end)
  (#make-range! "parameter.around" @parameter.inside @_end))

((apply_expression
  (arguments
   (argument) @parameter.inside
   .
   ","? @_end))
  (#make-range! "parameter.around" @parameter.inside @_end))

((dot_apply_expression
  (argument) @parameter.inside
  .
  ","? @_end)
  (#make-range! "parameter.around" @parameter.inside @_end))

((dot_dot_apply_expression
  (argument) @parameter.inside
  .
  ","? @_end)
  (#make-range! "parameter.around" @parameter.inside @_end))

((array_expression
  (_) @parameter.inside
  .
  ","? @_end)
  (#make-range! "parameter.around" @parameter.inside @_end))

; Comments

(comment) @comment.around
(block_comment) @comment.around
