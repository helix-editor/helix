; inherits: c

(foreach_statement
 body: (_) @indent
  (#not-kind-eq? @indent "compound_statement")
  (#set! "scope" "all"))

(foreach_instance_statement
  body: (_) @indent
  (#not-kind-eq? @indent "compound_statement")
  (#set! "scope" "all"))
