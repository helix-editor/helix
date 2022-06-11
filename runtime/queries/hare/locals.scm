(unit) @local.scope

(function_declaration) @local.scope

(global_binding
  (identifier) @local.definition)
(constant_binding 
  (identifier) @local.definition)
(type_bindings
  (identifier) @local.definition)

(function_declaration
  (prototype
    (parameter_list
      (parameters
        (parameter
          (name) @local.definition)))))

(identifier) @local.reference

