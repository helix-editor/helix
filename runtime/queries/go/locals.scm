; Scopes

[
  (function_declaration)
  (method_declaration)
  (func_literal)
  (type_declaration)
  (block)
] @local.scope

; Definitions

(parameter_declaration (identifier) @local.definition.variable.parameter)
(variadic_parameter_declaration (identifier) @local.definition.variable.parameter)

(const_declaration
 (const_spec
  name: (identifier) @local.definition.constant))

(var_spec
 name: (identifier) @local.definition.variable)

(short_var_declaration
 left: (expression_list (identifier) @local.definition.variable))

(range_clause
 left: (expression_list (identifier) @local.definition.variable))

; Bound identifier of `switch v := x.(type)`.
(type_switch_statement
 alias: (expression_list (identifier) @local.definition.variable))

; References

(identifier) @local.reference

; Field names in struct literals are identifier rather than field_identifier,
; these cannot be locals.
(keyed_element . (literal_element (identifier) @_))
