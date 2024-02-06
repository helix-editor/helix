; Scopes
[
  (function_definition)
  (if_statement)
  (for_statement)
  (compound_statement)
] @local.scope

; Definitions
(declaration
  declarator: (identifier) @local.definition)
(parameter_declaration
  declarator: (identifier) @local.definition)
(pointer_declarator
  declarator: (identifier) @local.definition)
(array_declarator
  declarator: (identifier) @local.definition)
(init_declarator
  declarator: (identifier) @local.definition)

; References
(identifier) @local.reference
