[
  (compound_statement)
  (declaration_list)
  (field_declaration_list)
  (enumerator_list)
  (parameter_list)
  (init_declarator)
  (expression_statement)
] @indent

[
  "case"
  "}"
  "]"
  ")"
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

(parameter_list
  . (parameter_declaration) @anchor
  (#set! "scope" "tail")) @align
(argument_list
  . (_) @anchor
  (#set! "scope" "tail")) @align
; These are a bit opinionated since some people just indent binary/ternary expressions spanning multiple lines.
; Since they are only triggered when a newline is inserted into an already complete binary/ternary expression,
; this should happen rarely, so it is not a big deal either way.
; Additionally, adding these queries has the advantage of preventing such continuation lines from being used
; as the baseline when the `hybrid` indent heuristic is used (which is desirable since their indentation is so inconsistent). 
(binary_expression
  (#set! "scope" "tail")) @anchor @align
(conditional_expression
  "?" @anchor
  (#set! "scope" "tail")) @align
