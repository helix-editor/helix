; Scopes

[
  (function_item)
  (function_lit)
  (module_item)
  (let_expression)
  (let_block)
  (assign_block)
  (for_block)
  (intersection_for_block)
  (list_comprehension)
] @local.scope

; Definitions

; Parameters: a bare `identifier`, or `name = default` via an `assignment`.
(parameters
  (parameter
    (identifier) @local.definition.variable.parameter))
(parameters
  (parameter
    (assignment
      name: (identifier) @local.definition.variable.parameter)))

; let(x = ...) / assign(x = ...) / for(x = ...) bindings
(assignments
  (assignment
    name: (identifier) @local.definition.variable))

; References

(identifier) @local.reference

; Call targets are function/module names, not variable references.
(function_call
  name: (identifier) @_)
(module_call
  name: (identifier) @_)
