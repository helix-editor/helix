[
  ; zig
  (ArrayTypeStart)
  ; using ()
  (AsmExpr)
  (AsmOutputItem)
  (ByteAlign)
  (CallConv)
  (ContainerDeclType)
  (ErrorSetDecl)
  (FnCallArguments)
  (ForPrefix)
  (GroupedExpr)
  (IfPrefix)
  (ParamDeclList)
  (SwitchExpr)
  (WhileContinueExpr)
  (WhilePrefix)
  ; for align expressions
  (PtrTypeStart)

  ; using {}
  (Block)
  (BlockExpr)
  (FormatSequence)
  (InitList)

  ; using []
  (SliceTypeStart)
  (SuffixOp)

  ; zig uses || for captures
  (Payload         "|" @rainbow.bracket)
  (PtrPayload      "|" @rainbow.bracket)
  (PtrIndexPayload "|" @rainbow.bracket)
] @rainbow.scope

[
  "(" ")"
  "{" "}"
  "[" "]"
] @rainbow.bracket
