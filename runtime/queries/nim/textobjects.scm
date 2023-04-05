(routine
  (block) @function.inside) @function.around

; @class.inside (types?)
; @class.around

; paramListSuffix is strange and i do not understand it
(paramList
  (paramColonEquals) @parameter.inside) @parameter.around

(comment) @comment.inside
(multilineComment) @comment.inside
(docComment) @comment.inside
(multilineDocComment) @comment.inside

(comment)+ @comment.around
(multilineComment) @comment.around
(docComment)+ @comment.around
(multilineDocComment) @comment.around
