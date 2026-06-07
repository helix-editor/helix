; Indent query for Nix.
;
; Nix is an expression language with no statement blocks. Indentation comes from
; the bracketed scopes ({…} attrsets, […] lists, (…) groups, {…} parameter
; sets), the `let … in` and `if … then … else` forms, multi-line function
; application, and the continuation of a binding whose value spills onto a
; following line.

; One level per bracketed scope; the matching close token dedents.
[
  (attrset_expression)
  (rec_attrset_expression)
  (let_attrset_expression)
  (list_expression)
  (parenthesized_expression)
  (formals)
] @indent

[
  "}"
  ")"
  "]"
] @outdent

; A binding value carried onto the line(s) after `=`. It shares the `=` line
; with a same-line bracket, so `x = { … }` is not indented twice.
(binding) @indent

; `let … in`: indent everything inside the `let`, then pull `in` and the body
; back so only the bindings stay indented. Indenting the whole `let_expression`
; (rather than its `binding_set`) also gives the right indent when a newline is
; typed straight after `let`, before any binding exists — the cursor is still
; inside `let_expression`, whereas the `binding_set` does not yet exist.
(let_expression) @indent
(let_expression "in" @outdent)
(let_expression body: (_) @outdent)

; `if … then … else`: indent each branch. scope "all" covers a branch written on
; its own line. An `else if` is skipped — the nested `if` indents its own
; branches, so the chain stays flat instead of stair-stepping.
(if_expression
  consequence: (_) @indent
  (#set! "scope" "header"))
(if_expression
  alternative: (_) @indent
  (#not-kind-eq? @indent "if_expression")
  (#set! "scope" "header"))

; Function application: arguments carried onto following lines. Nested
; applications share a line, so they collapse to a single level.
(apply_expression) @indent

; Indented strings are literal text (often embedded scripts); preserve their
; interior verbatim rather than reflowing it as code.
(indented_string_expression) @opaque
