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

(annotation_definition_identifier) @local.definition

(const_identifier) @local.definition

(enum (enum_identifier) @local.definition)

[
  (enum_member)
  (field_identifier)
] @local.definition

(method_identifier) @local.definition

(namespace) @local.definition

[
  (param_identifier)
  (return_identifier)
] @local.definition

(group (type_identifier) @local.definition)

(struct (type_identifier) @local.definition)

(union (type_identifier) @local.definition)

(interface (type_identifier) @local.definition)

; Generics Related (don't know how to combine these)

(struct
  (generics
    (generic_parameters
      (generic_identifier) @local.definition)))

(interface
  (generics
    (generic_parameters
      (generic_identifier) @local.definition)))

(method
  (implicit_generics
    (implicit_generic_parameters
      (generic_identifier) @local.definition)))

(method
  (generics
    (generic_parameters
      (generic_identifier) @local.definition)))

(annotation
  (generics
    (generic_parameters
      (generic_identifier) @local.definition)))

(replace_using
  (generics
    (generic_parameters
      (generic_identifier) @local.definition)))

(return_type
  (generics
    (generic_parameters
      (generic_identifier) @local.definition)))
