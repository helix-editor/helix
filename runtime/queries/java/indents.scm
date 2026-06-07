[
  (class_body)
  (enum_body)
  (interface_body)
  (constructor_body)
  (annotation_type_body)
  (module_body)
  (block)
  (switch_block)
  ; statements after a `case`/`default` label (the group also holds the label,
  ; so the default tail scope indents only the body lines, not the label)
  (switch_block_statement_group)
  (array_initializer)
  (argument_list)
  (formal_parameters)
  (annotation_argument_list)
  (element_value_array_initializer)
] @indent

[
  "}"
  ")"
  "]"
] @outdent

; Single statement after if/while/for without braces. Capture the body child
; naturally; on a typed newline the engine descends into it.
(if_statement
  consequence: (_) @indent
  (#not-kind-eq? @indent "block")
  (#set! "scope" "all"))
; Braceless `else` body (the body is the alternative field). Skip `else if`
; (alternative is an if_statement) and braced bodies.
(if_statement
  alternative: (_) @indent
  (#not-kind-eq? @indent "block")
  (#not-kind-eq? @indent "if_statement")
  (#set! "scope" "all"))
(while_statement
  body: (_) @indent
  (#not-kind-eq? @indent "block")
  (#set! "scope" "all"))
(for_statement
  body: (_) @indent
  (#not-kind-eq? @indent "block")
  (#set! "scope" "all"))

(string_literal) @opaque
