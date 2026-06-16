; Scopes

[
  (function_definition)
  (function_declaration)
  (block)
] @local.scope

; Definitions

(parameter_declaration
  name: (identifier) @local.definition.variable.parameter)

(variable_declaration
  name: (identifier) @local.definition.variable)
(old_variable_declaration
  name: (identifier) @local.definition.variable)

; References

(identifier) @local.reference

; Member/field names are not variable references.
(field_access
  field: (identifier) @_)
(scope_access
  field: (identifier) @_)

; Function names in call position are not variable references.
(call_expression
  function: (identifier) @_)

; Named argument labels are not variable references.
(named_arg
  arg_name: (identifier) @_)
