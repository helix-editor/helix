; Scopes
(block) @local.scope
(database) @local.scope

; Definitions
(floating_stmt
  (label) @local.definition.variable)

(essential_stmt
  (label) @local.definition.variable)

(axiom_stmt
  (label) @local.definition.function)

(provable_stmt
  (label) @local.definition.function)

(variable_stmt
  (variable) @local.definition.variable)

(constant_stmt
  (constant) @local.definition.constant)

; References in proofs
(uncompressed_proof
  (label) @local.reference)
