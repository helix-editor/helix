;; Upstream: https://github.com/tenzir/tree-sitter-tql/blob/main/queries/tql/indents.scm

[
  (then_block)
  (else_block)
  (pipeline_block)
  (match_statement)
  (match_arm)
  (list)
  (record)
] @indent

[
  "}"
  "]"
] @outdent
