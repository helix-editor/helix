[
  (block)
  (fn_stmt)
  (local_fn_stmt)
  (anon_fn)
  (for_range_stmt)
  (for_in_stmt)
] @local.scope

(_
  parameter_name: (name) @local.definition
)

(binding
  variable_name: (name) @local.definition
)

(var 
  variable_name: (name) @local.reference
)

; (call_stmt
;   .
;   method_table: (name) @local.reference
; )
