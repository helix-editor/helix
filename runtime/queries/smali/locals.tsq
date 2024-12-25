[
  (class_directive)
  (expression)
  (annotation_directive)
  (array_data_directive)
  (method_definition)
  (packed_switch_directive)
  (sparse_switch_directive)
  (subannotation_directive)
] @local.scope

[
  (identifier)
  (class_identifier)
  (label)
  (jmp_label)
] @local.reference

(enum_reference
  (field_identifier) @local.definition)

((field_definition
  (access_modifiers) @_mod
  (field_identifier) @local.definition)
  (#eq? @_mod "enum"))

(field_definition
  (field_identifier) @local.definition
  (field_type) @local.definition)

(annotation_key) @local.definition

(method_definition
  (method_signature (method_identifier) @local.definition))

(param_identifier) @local.definition

(annotation_directive
  (class_identifier) @local.definition)

(class_directive
  (class_identifier) @local.definition)
