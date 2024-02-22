; Scopes
;-------

[
  (ce_expression)
  (module_defn)
  (for_expression)
  (do_expression)
  (fun_expression)
  (function_expression)
  (try_expression)
  (match_expression)
  (elif_expression)
  (if_expression)
] @local.scope

; Definitions
;------------

(function_or_value_defn) @local.definition

; References
;-----------

(identifier) @local.reference
