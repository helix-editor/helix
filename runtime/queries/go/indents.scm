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

; Handle partially typed code.
(communication_case ":" @indent @extend)
(expression_case ":" @indent @extend)
(default_case ":" @indent @extend)
(type_case ":" @indent @extend)

; Labels (`Loop:`, `done:`) are de-indented one level by gofmt.
; Capturing the label_name (a child, not an ancestor of the labeled body)
; outdents only the label line.
(labeled_statement
  (label_name) @outdent)

; Raw string literals (backticks) span lines and are literal content.
(raw_string_literal) @opaque
