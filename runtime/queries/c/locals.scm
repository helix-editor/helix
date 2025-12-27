;; Scopes
(function_definition) @local.scope

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
