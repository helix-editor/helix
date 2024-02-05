; inherits: c

; Scopes
(template_function
  name: (identifier) @local.definition) @local.scope

[
  (foreach_statement)
  (template_declaration)
  (unmasked_statement)
] @local.scope

; Definitions
(reference_declarator
  (identifier) @local.definition)
(type_parameter_declaration
  (type_identifier) @local.definition)

(foreach_statement
  range_start: (assignment_expression
    left: (identifier) @local.definition))
(foreach_instance_statement
  initializer: (identifier) @local.definition)

; References
(type_identifier) @local.reference
