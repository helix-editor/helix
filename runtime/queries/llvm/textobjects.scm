(define
  body: (_) @function.inside) @function.around

(struct_type
  (struct_body) @class.inside) @class.around

(packed_struct_type
  (struct_body) @class.inside) @class.around

(array_type
  (array_vector_body) @class.inside) @class.around

(vector_type
  (array_vector_body) @class.inside) @class.around

(argument) @parameter.inside

(comment) @comment.inside

(comment)+ @comment.around
