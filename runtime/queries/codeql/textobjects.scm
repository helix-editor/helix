(qldoc) @comment.around
(block_comment) @comment.around
(line_comment) @comment.inside
(line_comment)+ @comment.around

(classlessPredicate
  ((varDecl) @parameter.inside . ","?) @parameter.around
  (body "{" (_)* @function.inside "}")) @function.around
(memberPredicate
  ((varDecl) @parameter.inside . ","?) @parameter.around
  (body "{" (_)* @function.inside "}")) @function.around

(dataclass
  ("{" (_)* @class.inside "}")?) @class.around
(datatype) @class.around
(datatypeBranch) @class.around
