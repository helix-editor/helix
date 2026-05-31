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
  (parameter_list)
  (field_declaration_list)
  (block)
  (var_declaration)
  (selector_expression)
  (binary_expression)
] @indent

[
  "]"
  ")"
] @outdent

; Switches and selects aren't indented, only their case bodies are.
; Outdent all closing braces except those closing switches or selects.
(
    (_ "}" @outdent) @outer
    (#not-kind-eq? @outer "select_statement")
    (#not-kind-eq? @outer "type_switch_statement")
    (#not-kind-eq? @outer "expression_switch_statement")
)

; Starting a line after a new case should indent.
[
  (communication_case)
  (expression_case)
  (default_case)
  (type_case)
] @extend

; Handle ERROR nodes for when auto-pairs is disabled.
; Typing an opening delimiter without a closing one produces an ERROR node.
(ERROR "{") @indent @extend
(ERROR "(") @indent
(ERROR "[") @indent

; Labels (`Loop:`, `done:`) are de-indented one level by gofmt.
; Capturing the label_name (a child, not an ancestor of the labeled body)
; outdents only the label line.
(labeled_statement
  (label_name) @outdent)
