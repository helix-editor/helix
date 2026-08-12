(identifier) @local.reference

[
  (namespace)
  (named_module)
  (function_or_value_defn)
  (fun_expression)
  (for_expression)
  (match_expression)
  (rule)
] @local.scope

(function_declaration_left
  .
  ((_) @local.definition.function))

; Parameters live in `argument_patterns`, which appears under function
; declarations, lambdas (`fun_expression`) and property accessors alike. The
; identifier can sit several pattern wrappers deep (paren/typed/tuple/as), so
; match it at any of those depths.
(argument_patterns
  [
   (_ (identifier) @local.definition.variable.parameter)
   (_ (_ (identifier) @local.definition.variable.parameter))
   (_ (_ (_ (identifier) @local.definition.variable.parameter)))
   (_ (_ (_ (_ (identifier) @local.definition.variable.parameter))))
   (_ (_ (_ (_ (_ (identifier) @local.definition.variable.parameter)))))
   (_ (_ (_ (_ (_ (_ (identifier) @local.definition.variable.parameter))))))
  ])

; `for i in ...` loop variable.
(for_expression
  (identifier) @local.definition.variable)

; The `field` of `a.b` is member access, not a value reference.
(dot_expression
  field: (long_identifier_or_op (identifier) @_))
(dot_expression
  field: (long_identifier_or_op (long_identifier (identifier) @_)))
