[
  (if)
  (when)
  (elif_branch)
  (else_branch)
  (of_branch) ; note: not case_statement
  (block)
  (while)
  (for)
  (try)
  (except_branch)
  (finally_branch)
  (defer)
  (static_statement)
  (proc_declaration)
  (func_declaration)
  (iterator_declaration)
  (converter_declaration)
  (method_declaration)
  (template_declaration)
  (macro_declaration)
  (symbol_declaration)
] @indent
;; increase the indentation level

[
  (if)
  (when)
  (elif_branch)
  (else_branch)
  (of_branch) ; note: not case_statement
  (block)
  (while)
  (for)
  (try)
  (except_branch)
  (finally_branch)
  (defer)
  (static_statement)
  (proc_declaration)
  (func_declaration)
  (iterator_declaration)
  (converter_declaration)
  (method_declaration)
  (template_declaration)
  (macro_declaration)
  (symbol_declaration)
] @extend
;; ???

[
  (return_statement)
  (raise_statement)
  (yield_statement)
  (break_statement)
  (continue_statement)
] @extend.prevent-once
;; end a level of indentation while staying indented

[
  ")" ; tuples
  "]" ; arrays, seqs
  "}" ; sets
] @outdent
;; end a level of indentation and unindent the line
