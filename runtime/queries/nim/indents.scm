[
  (typeDef)
  (ifStmt)
  (whenStmt)
  (elifStmt)
  (elseStmt)
  (ofBranch) ; note: not caseStmt
  (whileStmt)
  (tryStmt)
  (tryExceptStmt)
  (tryFinallyStmt)
  (forStmt)
  (blockStmt)
  (staticStmt)
  (deferStmt)
  (asmStmt)
  ; exprStmt?
] @indent
;; increase the indentation level

[
  (ifStmt)
  (whenStmt)
  (elifStmt)
  (elseStmt)
  (ofBranch) ; note: not caseStmt
  (whileStmt)
  (tryStmt)
  (tryExceptStmt)
  (tryFinallyStmt)
  (forStmt)
  (blockStmt)
  (staticStmt)
  (deferStmt)
  (asmStmt)
  ; exprStmt?
] @extend
;; ???

[
  (returnStmt)
  (raiseStmt)
  (yieldStmt)
  (breakStmt)
  (continueStmt)
] @extend.prevent-once
;; end a level of indentation while staying indented

[
  ")" ; tuples
  "]" ; arrays, seqs
  "}" ; sets
] @outdent
;; end a level of indentation and unindent the line
