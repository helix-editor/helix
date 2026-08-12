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
  name: (type_identifier) @local.definition.type)

(asset_declaration
  name: (type_identifier) @local.definition.type)

(participant_declaration
  name: (type_identifier) @local.definition.type)

(transaction_declaration
  name: (type_identifier) @local.definition.type)

(event_declaration
  name: (type_identifier) @local.definition.type)

(enum_declaration
  name: (type_identifier) @local.definition.type)

(scalar_declaration
  name: (type_identifier) @local.definition.type)

(map_declaration
  name: (type_identifier) @local.definition.type)

; Property declarations define properties
(string_field
  name: (identifier) @local.definition.variable.other.member)

(boolean_field
  name: (identifier) @local.definition.variable.other.member)

(datetime_field
  name: (identifier) @local.definition.variable.other.member)

(integer_field
  name: (identifier) @local.definition.variable.other.member)

(long_field
  name: (identifier) @local.definition.variable.other.member)

(double_field
  name: (identifier) @local.definition.variable.other.member)

(object_field
  name: (identifier) @local.definition.variable.other.member)

(relationship_field
  name: (identifier) @local.definition.variable.other.member)

(enum_property
  name: (identifier) @local.definition.variable.other.member)

; References
; ----------

; Type references
(extends_clause
  (type_identifier) @local.reference)

(object_field
  type: (type_identifier) @local.reference)

(relationship_field
  type: (type_identifier) @local.reference)
