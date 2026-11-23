; Scopes

[
  (source_file)
  (block)
  (function_declaration)
  (method_declaration)
  (lambda)
  (tree_body)
  (for_statement)
  (catch_clause)
  (match_arm)
] @local.scope

; Definitions

(parameter
  name: (identifier) @local.definition.variable.parameter)

(lambda_parameter
  name: (identifier) @local.definition.variable.parameter)

(variable_declaration
  name: (identifier) @local.definition.variable)

(plant_binding
  name: (identifier) @local.definition.variable)

(for_statement
  variable: (identifier) @local.definition.variable)

(catch_clause
  name: (identifier) @local.definition.variable)

(constant_declaration
  name: (identifier) @local.definition.constant)

; References

(identifier) @local.reference
