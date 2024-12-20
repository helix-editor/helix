; Scopes
(module (_) @local.scope)

(function
  body: (_) @local.scope)

; Definitions
(assign
  lhs: (identifier) @local.definition)

(variable
  (identifier) @local.definition)

(arg
  (identifier) @local.definition)

(arg
  (variable (identifier)) @local.definition)

(import_item
  (identifier) @local.definition)

(entry_block
  (identifier) @local.definition)

(entry_inline
  (identifier) @local.definition)

; References
(identifier) @local.reference
