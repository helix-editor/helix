[
  (repeat_stmt)
  (for_in_stmt)
  (for_range_stmt)
  (do_stmt)
  (while_stmt)
  (if_stmt)
  ; (else_clause)
  ; (elseif_clause)
  (fn_stmt)
  (local_fn_stmt)
  (anon_fn)
  (bindinglist)
  (varlist)
  (paramlist)
  (paramtypelist)
  (table)
  (cast)
  (tbtype)
  (dyntype)
  (bintype)
  (wraptype)
  (typepack)
  (attribute)
  (parattr)
  (littable)
] @indent

(
  [
    (arglist)
  ] @indent

)

(
  [
    (explist)
    (interp_exp)
  ] @indent.always

)

[
  "until"
  "end"
  ")"
  "}"
  "]"
] @outdent

[
  (interp_brace_close)
] @outdent.always

(ret_stmt
  "return" @expr-start
  .
  (_) @indent
  (#not-same-line? @indent @expr-start)

)

(field
  "[" @expr-start
  .
  field_indexer: (_) @indent
  (#not-same-line? @expr-start @indent)

)

(_
  (_) @expr-start
  .
  assign_symbol: _ @indent
  .
  (_) @expr-end
  (#not-same-line? @indent @expr-start)
  (#same-line? @expr-end @indent)

)

(_
  (_) @expr-start
  .
  assign_symbol: _ @assign-sym
  .
  (_) @indent
  (#same-line? @expr-start @assign-sym)
  (#not-same-line? @assign-sym @indent)

)

(ifexp
  [
    "if"
    "then"
    "elseif"
    "else"
  ] @expr-start
  .
  (_) @indent.always

  (#not-same-line? @indent.always @expr-start)
)

(fntype
  (paramtypelist) @expr-start
  return_type: (_) @indent
  (#not-same-line? @expr-start @indent)

)

(exp_wrap
  (_) @expr-content
  (#not-same-line? @indent @expr-content)
  (#not-kind-eq? @expr-content "ifexp")
) @indent

(else_clause
  "else" @outdent
)

(elseif_clause
  "elseif" @outdent
)
