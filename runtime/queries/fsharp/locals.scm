(identifier) @local.reference

[
  (namespace)
  (named_module)
  (function_or_value_defn)
] @local.scope

(function_declaration_left
  .
  ((_) @local.definition.function)
  ((argument_patterns
    [
     (_ (identifier) @local.definition.variable.parameter)
     (_ (_ (identifier) @local.definition.variable.parameter))
     (_ (_ (_ (identifier) @local.definition.variable.parameter)))
     (_ (_ (_ (_ (identifier) @local.definition.variable.parameter))))
     (_ (_ (_ (_ (_ (identifier) @local.definition.variable.parameter)))))
     (_ (_ (_ (_ (_ (_ (identifier) @local.definition.variable.parameter))))))
    ])
  ))
