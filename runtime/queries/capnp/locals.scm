; Scopes

[
  (message)
  (annotation_targets)
  (const_list)
  (enum)
  (interface)
  (implicit_generics)
  (generics)
  (group)
  (method_parameters)
  (named_return_types)
  (struct)
  (struct_shorthand)
  (union)
] @local.scope

; References

[
  (extend_type)
  (field_type)
] @local.reference
(custom_type (type_identifier) @local.reference)
(custom_type
  (generics
    (generic_parameters 
      (generic_identifier) @local.reference)))

; Definitions

[
  (param_identifier)
  (return_identifier)
] @local.definition.variable.parameter
