(Decl (FnProto)
  (_) @function.inside) @function.around

(TestDecl (_) @test.inside) @test.around

; matches all of: struct, enum, union
; this unfortunately cannot be split up because
; of the way struct "container" types are defined
(Decl (VarDecl (ErrorUnionExpr (SuffixExpr (ContainerDecl
    (_) @class.inside))))) @class.around

(Decl (VarDecl (ErrorUnionExpr (SuffixExpr (ErrorSetDecl
    (_) @class.inside))))) @class.around

(ParamDeclList
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

[
  (doc_comment)
  (line_comment)
] @comment.inside
(line_comment)+ @comment.around
(doc_comment)+ @comment.around
