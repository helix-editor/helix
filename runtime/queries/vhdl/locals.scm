; Scopes

[
  (subprogram_definition)
  (process_statement)
] @local.scope

; Definitions

; Subprogram parameters: each interface declaration's identifier_list holds the
; parameter name(s), reachable regardless of the constant:/generic: field label.
(parameter_list_specification
  (interface_list
    (_
      (identifier_list
        (identifier) @local.definition.variable.parameter))))

; Locally declared signals/variables/constants.
(signal_declaration
  (identifier_list
    (identifier) @local.definition.variable))
(variable_declaration
  (identifier_list
    (identifier) @local.definition.variable))
(constant_declaration
  (identifier_list
    (identifier) @local.definition.constant))

; References

; A use of a name is `(name (identifier) ...)`; the base identifier may resolve.
; Member identifiers live inside (selection ...) and are excluded by this shape.
(name
  (identifier) @local.reference)
