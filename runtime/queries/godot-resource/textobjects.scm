(section 
  (identifier) 
  (_)
  (property) @class.inside
) @class.around

(attribute 
  (identifier)
  (_) @parameter.inside) @parameter.around

(property 
  (path)
  (_) @entry.inside) @entry.around

(pair 
  (_) @entry.inside) @entry.around

(array
  (_) @entry.around)

(comment) @comment.inside

(comment)+ @comment.around
