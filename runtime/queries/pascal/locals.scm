; Scopes

[
  (declProc)
  (block)
] @local.scope

; Definitions

(declArg
  name: (identifier) @local.definition.variable.parameter)

(declVar
  name: (identifier) @local.definition.variable)

; References

(identifier) @local.reference

; The rhs of `a.b` is a member access, not a variable reference.
(exprDot
  rhs: (identifier) @_)

; Procedure/function names in call position are not variable references.
(exprCall
  entity: (identifier) @_)
