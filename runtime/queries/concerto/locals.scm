; Concerto Language - Locals Queries (Helix)
; ============================================
; Helix-specific local scope/definition/reference tracking. For use in
; helix-editor/helix at runtime/queries/concerto/locals.scm
;
; Helix uses the same @local.scope, @local.definition, and @local.reference
; capture names as tree-sitter convention.

; Scopes
; ------

; Each declaration body creates a new scope
(concept_declaration) @local.scope
(asset_declaration) @local.scope
(participant_declaration) @local.scope
(transaction_declaration) @local.scope
(event_declaration) @local.scope
(enum_declaration) @local.scope
(map_declaration) @local.scope

; Definitions
; -----------

; Type declarations define types
(concept_declaration
  name: (type_identifier) @local.definition)

(asset_declaration
  name: (type_identifier) @local.definition)

(participant_declaration
  name: (type_identifier) @local.definition)

(transaction_declaration
  name: (type_identifier) @local.definition)

(event_declaration
  name: (type_identifier) @local.definition)

(enum_declaration
  name: (type_identifier) @local.definition)

(scalar_declaration
  name: (type_identifier) @local.definition)

(map_declaration
  name: (type_identifier) @local.definition)

; Property declarations define properties
(string_field
  name: (identifier) @local.definition)

(boolean_field
  name: (identifier) @local.definition)

(datetime_field
  name: (identifier) @local.definition)

(integer_field
  name: (identifier) @local.definition)

(long_field
  name: (identifier) @local.definition)

(double_field
  name: (identifier) @local.definition)

(object_field
  name: (identifier) @local.definition)

(relationship_field
  name: (identifier) @local.definition)

(enum_property
  name: (identifier) @local.definition)

; References
; ----------

; Type references
(extends_clause
  (type_identifier) @local.reference)

(object_field
  type: (type_identifier) @local.reference)

(relationship_field
  type: (type_identifier) @local.reference)
