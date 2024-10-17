(class
  body: (_) @class.inside) @class.around

(multiclass
  body: (_) @class.inside) @class.around

(_ argument: _ @parameter.inside)

[
  (comment)
  (multiline_comment)
] @comment.inside

(comment)+ @comment.around

(multiline_comment) @comment.around
