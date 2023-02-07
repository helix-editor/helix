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
  (type_switch_statement)
  (expression_switch_statement)
  (var_declaration)
] @indent

[
  "]"
  ")"
] @outdent

((_ "}" @outdent) @outer (#not-kind-eq? @outer "select_statement"))
(communication_case) @extend
