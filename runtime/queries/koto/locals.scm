; Scopes
(module (_) @local.scope)

(function
  body: (_) @local.scope)

; Definitions
(arg
  (identifier) @local.definition.variable.parameter)

(arg
  (variable (identifier)) @local.definition.parameter)

(import_item
  (identifier) @local.definition.namespace)

(entry_block
  (identifier) @local.definition.variable.other.member)

(entry_inline
  (identifier) @local.definition.variable.other.member)

; References
(identifier) @local.reference
