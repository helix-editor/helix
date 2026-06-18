; inherits: c

; C++-specific scopes on top of c's function_definition / declaration scopes.
[
  (lambda_expression)
  (namespace_definition)
  (class_specifier)
  (for_range_loop)
] @local.scope

; C++-only parameter forms (c only has parameter_declaration).
(optional_parameter_declaration
  declarator: (identifier) @local.definition.variable.parameter)
(variadic_parameter_declaration
  declarator: (variadic_declarator (identifier) @local.definition.variable.parameter))

; Template type parameters.
(type_parameter_declaration
  (type_identifier) @local.definition.type)
