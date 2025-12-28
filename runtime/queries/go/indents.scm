[
  (import_declaration)
  (const_declaration)
  (type_declaration)
  (type_spec)
  (func_literal)
  (literal_value)
  (literal_element)
  (keyed_element)
  (expression_case)
  (default_case)
  (type_case)
  (communication_case)
  (argument_list)
  (field_declaration_list)
  (block)
  (var_declaration)
] @indent

[
  "]"
  ")"
] @outdent

; Switches and selects aren't indented, only their case bodies are.
; Outdent all closing braces except those closing switches or selects.
(
    (_ "}" @outdent) @_outer
    (#not-kind-eq? @_outer "select_statement")
    (#not-kind-eq? @_outer "type_switch_statement")
    (#not-kind-eq? @_outer "expression_switch_statement")
)

; Starting a line after a new case should indent.
[
  (communication_case)
  (expression_case)
  (default_case)
  (type_case)
] @extend
