; Scopes

[
  (function_declaration)
  (method_declaration)
  (type_declaration)
  (block)
] @local.scope

; Definitions

(parameter_declaration (identifier) @local.definition.variable.parameter)
(variadic_parameter_declaration (identifier) @local.definition.variable.parameter)

(const_declaration
 (const_spec
  name: (identifier) @local.definition.constant))

; References

(identifier) @local.reference

; Field names in struct literals are identifier rather than field_identifier,
; these cannot be locals.
(keyed_element . (literal_element (identifier) @variable.other.member))
