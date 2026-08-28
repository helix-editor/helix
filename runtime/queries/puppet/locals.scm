; Scopes

[
  (block)
  (defined_resource_type)
  (function_declaration)
  (attribute_type_entry)
  (class_definition)
  (node_definition)
  (resource_declaration)
  (selector)
  (iterator_statement)
  (case_statement)
  (hash)
  (array)
] @local.scope

; Definitions
;
; Only variables and parameters are tracked: types, functions and namespaces
; already get their correct highlight straight from highlights.scm, and their
; identifier segments are always the innermost node at their position, so
; resolving them again here would have no visible effect.

(parameter (variable (identifier) @local.definition.variable.parameter))

(assignment . (variable (identifier) @local.definition.variable))

; References

(identifier) @local.reference

; Attribute names are hash-like keys, not variable references.
(attribute name: (identifier) @_)
