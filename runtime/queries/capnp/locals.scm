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
] @scope

[
  (extend_type)
  (field_type)
] @reference
(custom_type (type_identifier) @reference)
(custom_type
  (generics
    (generic_parameters 
      (generic_identifier) @reference)))

(annotation_definition_identifier) @definition

(const_identifier) @definition.constant

(enum (enum_identifier) @definition.enum)

[
  (enum_member)
  (field_identifier)
] @definition.field

(method_identifier) @definition.method

(namespace) @definition.namespace

[
  (param_identifier)
  (return_identifier)
] @definition.parameter

(group (type_identifier) @definition.type)

(struct (type_identifier) @definition.type)

(union (type_identifier) @definition.type)

(interface (type_identifier) @definition.type)

; Generics Related (don't know how to combine these)

(struct
  (generics
    (generic_parameters
      (generic_identifier) @definition.parameter)))

(interface
  (generics
    (generic_parameters
      (generic_identifier) @definition.parameter)))

(method
  (implicit_generics
    (implicit_generic_parameters
      (generic_identifier) @definition.parameter)))

(method
  (generics
    (generic_parameters
      (generic_identifier) @definition.parameter)))

(annotation
  (generics
    (generic_parameters
      (generic_identifier) @definition.type)))

(replace_using
  (generics
    (generic_parameters
      (generic_identifier) @definition.type)))

(return_type
  (generics
    (generic_parameters
      (generic_identifier) @definition.type)))
