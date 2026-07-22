; Scopes

[
  (p_fun_decl)
  (anon_event_handler)
  (foreign_fun_decl)
  (pure_decl)
  (quant_expr)
] @local.scope

; Definitions

(fun_param
  name: (identifier) @local.definition.variable.parameter)

; References

(primitive_expr (identifier) @local.reference)
(var_lvalue name: (identifier) @local.reference)
