; Sema textobjects for Helix

; Function definitions
(list
  .
  (symbol) @_f
  .
  (_) @function.inside
  (#any-of? @_f "define" "def" "defun" "defn" "lambda" "fn" "defmacro")
) @function.around

; Agent/tool definitions
(list
  .
  (symbol) @_f
  .
  (_) @class.inside
  (#any-of? @_f "defagent" "deftool")
) @class.around

; Comments
(comment) @comment.inside
(comment)+ @comment.around
