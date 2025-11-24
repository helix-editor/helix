(function_declaration
  body: (_) @function.inside) @function.around

(test_declaration (_) (block) @test.inside) @test.around

; matches all of: struct, enum, union
; this unfortunately cannot be split up because
; of the way struct "container" types are defined
(variable_declaration (identifier) (struct_declaration
    (_) @class.inside)) @class.around

(variable_declaration (identifier) (enum_declaration
    (_) @class.inside)) @class.around

(variable_declaration (identifier) (enum_declaration
    (_) @class.inside)) @class.around

(parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(comment) @comment.inside
(comment)+ @comment.around
