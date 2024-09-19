(directive_term) @indent.zero

(clause_term) @indent.zero

(functional_notation
  (atom)
  (open_ct) @indent.begin
  (close) @indent.end)

(list_notation
  (open_list) @indent.begin
  (close_list) @indent.end)

(curly_bracketed_notation
  (open_curly) @indent.begin
  (close_curly) @indent.end)
