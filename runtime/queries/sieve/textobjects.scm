[
  (comment_line)
  (comment_block)
] @comment.inside

(comment_line)+ @comment.around

(comment_block) @comment.around

(arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(test_list
  (_) @entry.around)
