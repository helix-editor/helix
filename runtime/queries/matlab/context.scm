(function_definition
  (block (_) @context.end)
) @context

(while_statement
  (block (_) @context.end)
) @context

(for_statement
  (block (_) @context.end)
) @context

(if_statement
  (block (_) @context.end)
) @context

(elseif_clause
  (block (_) @context.end)
) @context

(else_clause
  (block (_) @context.end)
) @context

(switch_statement) @context

(case_clause
  (block (_) @context.end)
) @context

(otherwise_clause
  (block (_) @context.end)
) @context

(try_statement
  "try"
  (block (_) @context.end) @context
  "end")
(catch_clause
  "catch"
  (block (_) @context.end) @context)
