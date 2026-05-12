(lambda_expression
  (label) @parameter.inside
  (expression) @function.inside
) @function.around

(forall_expression
  (label) @parameter.inside
  (expression) @function.inside
) @function.around

(assert_expression
  (expression) @test.inside
) @test.around

[
  (block_comment_content)
  (line_comment_content)
] @comment.inside

[
  (block_comment)
  (line_comment)
] @comment.around
