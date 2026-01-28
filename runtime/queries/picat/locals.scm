; locals

[
  (predicate_definition)
  (function_definition)
  (actor_definition)
  (actor_definition)
] @local.scope

(import_declaration (_) @local.definition)

(module_declaration (_) @local.definition)

(binary_relational_expression left: (atom) @local.definition)

(parameters
  [(variable) @local.definition
   (atom) @local.definition
   (array_expression [(variable) @local.definition (atom) @local.definition])
   (list_expression [(variable) @local.definition (atom) @local.definition])
   (as_pattern_expression left: [(variable) @local.definition (atom) @local.definition])])

(arguments (argument [(variable) @local.reference (atom) @local.reference]))

[(variable) (atom)] @local.reference
