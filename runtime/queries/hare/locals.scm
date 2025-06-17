(sub_unit) @local.scope

(function_declaration) @local.scope
(compound_expression) @local.scope

(function_declaration
  (identifier) @local.definition.function)
(function_declaration
  (parameter (name) @local.definition.variable.parameter))

(identifier) @local.reference

