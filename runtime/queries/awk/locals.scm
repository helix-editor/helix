; Scopes

(func_def) @local.scope

; Definitions

; awk has no local declarations; extra params are the idiom for locals.
(func_def
  (param_list
    (identifier) @local.definition.variable.parameter))

; References

(identifier) @local.reference

; Function names in call position are not variable references.
(func_call
  name: (identifier) @_)
