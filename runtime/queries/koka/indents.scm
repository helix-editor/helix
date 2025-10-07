[
  (opexpr [index: (arguments) call: (arguments)]) ; Applications.
  (atom ["[" "("]) ; Lists and tuples.
  (funbody)
  (block)
  (constructor)
  (handlerexpr)
  (opclausex)
] @indent

[
  (typedecl
    [(typeid) (opdecls)]) ; Avoid matching single-operation effects.
  (externdecl)
  (matchexpr)
  (matchrule)

  ; For ifexprs, branches (once they exist) will contain blocks if they're
  ; indented so we just need to make sure the initial indent happens when we're
  ; creating them.
  "then"
  "else"
] @indent @extend

(matchrule "->" @indent @extend)

; Handling for error recovery.
(ERROR "fun") @indent @extend
(ERROR "match") @indent @extend
(ERROR "->" @indent.always @extend)

; Don't outdent on function parameter declarations.
(atom ")" @outdent @extend.prevent-once)

[
  "]"
  "}"
] @outdent @extend.prevent-once
