; Scopes       @local.scope
; -------------------------

[
  (static_function)
  (init_function)
  (bounced_function)
  (receive_function)
  (external_function)
  (function)
  (block_statement)
] @local.scope

; Definitions  @local.definition
; ------------------------------

(parameter
  name: (identifier) @local.definition.variable.parameter)

(constant
  name: (identifier) @local.definition.constant)

; References   @local.reference
; -----------------------------

(self) @local.reference

(value_expression (identifier) @local.reference)

(lvalue (identifier) @local.reference)
