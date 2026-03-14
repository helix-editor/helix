; Scopes
;-------

[
  (let_binding)
  (class_binding)
  (class_function)
  (method_definition)
  (fun_expression)
  (object_expression)
  (for_expression)
  (match_case)
  (attribute_payload)
] @local.scope

; Definitions
;------------

(value_pattern) @local.definition.variable.parameter

; References
;-----------

(value_path . (value_name) @local.reference)
