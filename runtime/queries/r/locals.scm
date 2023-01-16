; locals.scm

(function_definition) @local.scope

(formal_parameters (identifier) @local.definition)

(left_assignment name: (identifier) @local.definition)
(equals_assignment name: (identifier) @local.definition)
(right_assignment name: (identifier) @local.definition)

(identifier) @local.reference
