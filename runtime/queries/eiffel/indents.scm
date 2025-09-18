[
  (class_declaration)
  (feature_declaration)
  (feature_body)
  (notes)
  (precondition)
  (local_declarations)
  (postcondition)
  (check)
  (initialization)
  (iteration)
  (loop)
  (quantifier_loop)
  (then_part)
  (else_part)
  (then_part_expression)
  (else_part_expression)
  (multi_branch)
] @indent.begin
(exit_condition) @indent.branch
(loop_body) @indent.branch
(variant) @indent.branch
(invariant) @indent.branch
(loop "end" @indent.branch)
(class_declaration "class" @indent.branch)
(check "end" @indent.branch)
(class_declaration "end" @indent.branch)
(feature_clause "feature" @indent.branch)
(inheritance "inherit" @indent.branch)
(creation_clause "create" @indent.branch)
