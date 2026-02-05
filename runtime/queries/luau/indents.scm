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
  (#set! "scope" "all")
)

(
  [
    (explist)
    (interp_exp)
  ] @indent.always
  (#set! "scope" "all")
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
  "return" @_expr-start
  .
  (_) @indent
  (#not-same-line? @indent @_expr-start)
  (#set! "scope" "all")
)

(field
  "[" @_expr-start
  .
  field_indexer: (_) @indent
  (#not-same-line? @_expr-start @indent)
  (#set! "scope" "all")
)

(_
  (_) @_expr-start
  .
  assign_symbol: _ @indent
  .
  (_) @_expr-end
  (#not-same-line? @indent @_expr-start)
  (#same-line? @_expr-end @indent)
  (#set! "scope" "all")
)

(_
  (_) @_expr-start
  .
  assign_symbol: _ @_assign-sym
  .
  (_) @indent
  (#same-line? @_expr-start @_assign-sym)
  (#not-same-line? @_assign-sym @indent)
  (#set! "scope" "all")
)

(ifexp
  [
    "if"
    "then"
    "elseif"
    "else"
  ] @_expr-start
  .
  (_) @indent.always
  (#set! "scope" "all")
  (#not-same-line? @indent.always @_expr-start)
)

(fntype
  (paramtypelist) @_expr-start
  return_type: (_) @indent
  (#not-same-line? @_expr-start @indent)
  (#set! "scope" "all")
)

(exp_wrap
  (_) @_expr-content
  (#not-same-line? @indent @_expr-content)
  (#not-kind-eq? @_expr-content "ifexp")
) @indent

(else_clause
  "else" @outdent
)

(elseif_clause
  "elseif" @outdent
)
