(type_declaration
  (relations) @class.inside) @class.around

(condition_declaration
  body: (_) @function.inside) @function.around

(relations
  (definition) @entry.inside) @entry.around

(param 
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(comment) @comment.inside

(comment)+ @comment.around
