((list
  . (datum (symbol) @_define)
  . (datum (list))
  . (datum)* @function.inside) @function.around
  (#eq? @_define "define"))

((list
  . (datum (symbol) @_lambda)
  . (datum)
  . (datum)* @function.inside) @function.around
  (#eq? @_lambda "lambda"))

[(line_comment) (block_comment)] @comment.inside
(line_comment)+ @comment.around
(block_comment) @comment.around
