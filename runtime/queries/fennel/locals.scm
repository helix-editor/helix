; Scopes

[
  (fn_form)
  (lambda_form)
  (hashfn_form)
  (let_form)
  (each_form)
  (for_form)
  (collect_form)
  (icollect_form)
  (fcollect_form)
  (accumulate_form)
  (faccumulate_form)
] @local.scope

; Definitions

; Function parameters; rest_binding covers the trailing `& rest` form.
(sequence_arguments
  (symbol_binding) @local.definition.variable.parameter)
(sequence_arguments
  (rest_binding
    rhs: (symbol_binding) @local.definition.variable.parameter))

; (let [x ...] ...)
(let_vars
  (binding_pair
    lhs: (symbol_binding) @local.definition.variable))

; (local x ...) / (var x ...) / (global x ...)
[
  (local_form)
  (var_form)
  (global_form)
]
  (binding_pair
    lhs: (symbol_binding) @local.definition.variable)

; Loop bindings
(iter_body
  binding: (symbol_binding) @local.definition.variable)
(for_iter_body
  index: (symbol_binding) @local.definition.variable)
(accumulator_pair
  accumulator_binding: (symbol_binding) @local.definition.variable)

; References

(symbol) @local.reference
