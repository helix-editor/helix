[
  (AsmExpr)
  (AssignExpr)
  (Block)
  (BlockExpr)
  (ContainerDecl)
  (ErrorUnionExpr)
  (InitList)
  (Statement)
  (SwitchExpr)
  (TestDecl)
] @indent

[
  "}"
  "]"
  ")"
] @outdent

(IfExpression
  .
  (_) @expr-start
  condition: (_) @indent
  (#not-same-line? @indent @expr-start)
  (#set! "scope" "all")
)
