; Scopes

[
  (function_definition)
  (method_declaration)
  (anonymous_function)
  (arrow_function)
  (compound_statement)
] @local.scope

; Definitions

; PHP variables are `variable_name` ($foo); parameters share that node type so
; references match by text including the leading `$`.
(simple_parameter
  name: (variable_name) @local.definition.variable.parameter)
(variadic_parameter
  name: (variable_name) @local.definition.variable.parameter)
(property_promotion_parameter
  name: (variable_name) @local.definition.variable.parameter)

; References

(variable_name) @local.reference
