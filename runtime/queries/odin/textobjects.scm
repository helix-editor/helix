(procedure_declaration (identifier) (procedure (block) @function.inside)) @function.around
(procedure_declaration (identifier) (procedure (uninitialized) @function.inside)) @function.around
(overloaded_procedure_declaration (identifier) @function.inside) @function.around

(procedure_type (parameters (parameter (identifier) @parameter.inside) @parameter.around))
(procedure (parameters (parameter (identifier) @parameter.inside) @parameter.around))

((procedure_declaration
  (attributes (attribute "@" "(" (identifier) @attr_name ")"))
  (identifier) (procedure (block) @test.inside)) @test.around
 (#match? @attr_name "test"))

(comment) @comment.inside
(comment)+ @comment.around
(block_comment) @comment.inside
(block_comment)+ @comment.around

(struct_declaration (identifier) "::") @class.around
(enum_declaration (identifier) "::") @class.around
(union_declaration (identifier) "::") @class.around
(bit_field_declaration (identifier) "::") @class.around
(const_declaration (identifier) "::" [(array_type) (distinct_type) (bit_set_type) (pointer_type)]) @class.around
