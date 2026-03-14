[ "(" ")" "[" "]" "{" "}" ] @rainbow.bracket
[
  ; quote & unquote
  (quote_form)
  (unquote_form)

  ; bindings + variables
  (local_form)
  (var_form)
  (set_form)
  (global_form)

  ; let
  (let_form)
  (let_vars)

  ; functions
  (fn_form)
  (lambda_form)
  (hashfn_form)

  ; case & match
  (case_form)
  (case_catch)
  (case_guard)
  (case_guard_or_special)
  (match_form)

  ; case-try & match-try
  (case_try_form)
  (match_try_form)

  ; each
  (each_form)
  (iter_body)

  ; if
  (if_form)

  ; import-macros
  (import_macros_form)

  ; macro
  (macro_form)

  ; other
  (list)
  (list_binding)
  (sequence)
  (sequence_binding)
  (sequence_arguments)
  (table)
  (table_binding)
  (table_metadata)
] @rainbow.scope
