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

(IfStatement
  .
  (_) @expr-start
  (_) @indent
  (#not-same-line? @indent @expr-start)
  (#set! "scope" "all")
)
