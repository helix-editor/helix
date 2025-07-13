(function_clause
  pattern: (arguments (_)? @parameter.inside)
  body: (_) @function.inside) @function.around

(anonymous_function
  (stab_clause body: (_) @function.inside)) @function.around

(comment (comment_content) @comment.inside) @comment.around

; EUnit test names.
; (CommonTest cases are not recognizable by syntax alone.)
((function_clause
   name: (atom) @_name
   pattern: (arguments (_)? @parameter.inside)
   body: (_) @test.inside) @test.around
 (#match? @_name "_test$"))
