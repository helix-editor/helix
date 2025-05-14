; Scopes

[
  (function_declaration)
  (type_declaration)
  (block)
] @local.scope

; Definitions

(type_parameter_list
  (parameter_declaration
    name: (identifier) @local.definition.variable.parameter))

(parameter_declaration (identifier) @local.definition.variable.parameter)
(variadic_parameter_declaration (identifier) @local.definition.variable.parameter)

(const_declaration
 (const_spec
  name: (identifier) @local.definition.constant))

; References

(identifier) @local.reference
(field_identifier) @local.reference
(type_identifier) @local.reference
