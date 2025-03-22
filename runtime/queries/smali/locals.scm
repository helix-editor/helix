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

(method_definition
  (method_signature (method_identifier) @local.definition.function.method))

(param_identifier) @local.definition.variable.parameter
