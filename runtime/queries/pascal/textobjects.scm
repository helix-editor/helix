
(declType (declClass (declSection) @class.inside)) @class.around

(defProc body: (_) @function.inside) @function.around

(declArgs (_) @parameter.inside) @parameter.around
(exprArgs (_) @parameter.inside) @parameter.around

(comment) @comment.inside
(comment)+ @comment.around
