;; Scopes
(function_definition) @local.scope
(declaration) @local.scope

;; Definitions

; Parameters
; Up to 6 layers of declarators
(parameter_declaration
  (identifier) @local.definition.variable.parameter)
(parameter_declaration
  (_
    (identifier) @local.definition.variable.parameter))
(parameter_declaration
  (_
    (_
      (identifier) @local.definition.variable.parameter)))
(parameter_declaration
  (_
    (_
      (_
        (identifier) @local.definition.variable.parameter))))
(parameter_declaration
  (_
    (_
      (_
        (_
          (identifier) @local.definition.variable.parameter)))))
(parameter_declaration
  (_
    (_
      (_
        (_
          (_
            (identifier) @local.definition.variable.parameter))))))

;; References

(identifier) @local.reference

; A call's function name is not a variable reference; keep its class
; even when a same-named local is in scope.
(call_expression
  function: (identifier) @_)
