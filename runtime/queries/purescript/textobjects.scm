(comment) @comment.inside

[
  (decl_adt)
  (decl_type)
  (newtype)
] @class.around

((signature)? (function rhs:(_) @function.inside)) @function.around 
(exp_lambda) @function.around

(adt (type_variable) @parameter.inside)
(patterns (_) @parameter.inside)
