[
  (compound_statement)
  (field_declaration_list)
  (enumerator_list)
  (parameter_list)
  (init_declarator)
  (case_statement)
  (expression_statement)
] @indent

[
  "case"
  "}"
  "]"
] @outdent

(if_statement
  consequence: (_) @indent
  (#not-kind-eq? @indent "compound_statement")
  (#set! "scope" "all"))
(while_statement
  body: (_) @indent
  (#not-kind-eq? @indent "compound_statement")
  (#set! "scope" "all"))
(do_statement
  body: (_) @indent
  (#not-kind-eq? @indent "compound_statement")
  (#set! "scope" "all"))
(for_statement
  ")"
  (_) @indent
  (#not-kind-eq? @indent "compound_statement")
  (#set! "scope" "all"))
