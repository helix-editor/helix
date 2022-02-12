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

; The #not-match? is just required to exclude compound statements. 
; It would be nice to do this somehow without regexes
(if_statement
  consequence: (_) @indent
  (#not-match? @indent "\\\{*\\\}")
  (#set! "scope" "all"))
(while_statement
  body: (_) @indent
  (#not-match? @indent "\\\{*\\\}")
  (#set! "scope" "all"))
(do_statement
  body: (_) @indent
  (#not-match? @indent "\\\{*\\\}")
  (#set! "scope" "all"))
(for_statement
  ")"
  (_) @indent
  (#not-match? @indent "\\\{*\\\}")
  (#set! "scope" "all"))
