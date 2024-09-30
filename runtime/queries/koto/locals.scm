; Scopes
(module (_) @local.scope)

(function
  body: (_) @local.scope)

; Definitions
(assign
  lhs: (identifier) @local.definition.var)

(variable
  (identifier) @local.definition.var)

(arg
  (identifier) @local.definition.parameter)

(arg
  (variable (identifier)) @local.definition.parameter)

(import_item
  (identifier) @local.definition.import)

(entry_block
  (identifier) @local.definition.field)

(entry_inline
  (identifier) @local.definition.field)

; References
(identifier) @local.reference
