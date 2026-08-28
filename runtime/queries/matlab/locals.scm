(function_definition name: (identifier) @local.definition.function ?) @local.scope
(function_arguments (identifier)* @local.definition.variable.parameter)

(lambda (arguments (identifier) @local.definition.variable.parameter)) @local.scope

(identifier) @local.reference

; A call's function name and a field member name are not variable references;
; cancel resolution so a same-named local doesn't capture them.
(function_call
  name: (identifier) @_)
(field_expression
  field: (identifier) @_)
