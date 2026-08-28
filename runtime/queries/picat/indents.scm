[
  (array_expression)
  (list_expression)
  (braced_goal)
  (arguments)
  (parameters)

  (predicate_rule)
  (predicate_fact)
  (function_rule)
  (function_fact)
  (action_rule)
  (nonbacktrackable_predicate_rule)

  (if_statement)
  (while_statement)
  (foreach_statement)
  (do_statement)
] @indent

[
  ")"
  "}"
  "]"
] @outdent

(if_statement "elseif" (_) @outdent)
(if_statement "else" (_) @outdent)
(while_statement "do" (_) @outdent)
(do_statement "while" (_) @outdent)
