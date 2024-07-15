(sub_unit) @local.scope

(function_declaration) @local.scope
(compound_expression) @local.scope

(global_binding
  (identifier) @local.definition)
(constant_binding
  (identifier) @local.definition)
(type_binding
  (identifier) @local.definition)

(function_declaration
  (identifier) @local.definition)
(function_declaration
  (parameter (name) @local.definition))

(identifier) @local.reference

