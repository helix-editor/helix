; locals

[
  (predicate_definition)
  (function_definition)
  (actor_definition)
  (actor_definition)
] @local.scope

(import_declaration (_) @local.definition.namespace)

(module_declaration (_) @local.definition.namespace)

(binary_relational_expression left: (atom) @local.definition.variable)

(parameters
  [(variable) @local.definition.variable.parameter
   (atom) @local.definition.variable.parameter
   (array_expression [(variable) (atom)] @local.definition.variable.parameter)
   (list_expression [(variable) (atom)] @local.definition.variable.parameter)
   (as_pattern_expression left: [(variable) (atom)] @local.definition.variable.parameter)])

(arguments (argument [(variable) @local.reference (atom) @local.reference]))

[(variable) (atom)] @local.reference
