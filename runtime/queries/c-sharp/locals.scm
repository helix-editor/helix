; Scopes

[
  (method_declaration)
  (local_function_statement)
  (constructor_declaration)
  (destructor_declaration)
  (operator_declaration)
  (lambda_expression)
  (anonymous_method_expression)
  (block)
] @local.scope

; Definitions

(parameter
  name: (identifier) @local.definition.variable.parameter)

(variable_declarator
  name: (identifier) @local.definition.variable)

(foreach_statement
  left: (identifier) @local.definition.variable)

; References

(identifier) @local.reference

; Discards: identifiers that look like references but aren't variables.

; `obj.Member` — the member name is not a local.
(member_access_expression
  name: (identifier) @_)

; Named argument `f(name: value)`.
(argument
  name: (identifier) @_)
