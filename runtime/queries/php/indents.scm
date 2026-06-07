[
  (array_creation_expression)
  (arguments)
  (formal_parameters)
  (compound_statement)
  (declaration_list)
  (binary_expression)
  (return_statement)
  (expression_statement)
  (switch_block)
  ; statements after a `case`/`default` label (the node also holds the label,
  ; so the default tail scope indents only the body lines, not the label)
  (case_statement)
  (default_statement)
  (anonymous_function_use_clause)
  (property_hook_list)
] @indent

[
  "}"
  ")"
  "]"
] @outdent

; Heredoc / nowdoc bodies are literal content.
[
  (heredoc)
  (nowdoc)
] @opaque
