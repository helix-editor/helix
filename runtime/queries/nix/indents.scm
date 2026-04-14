; Indent query for Nix.
;
; Helix uses @indent / @outdent / @align rather than nvim-treesitter's
; @indent.begin / @indent.end / @indent.branch captures.

; Nodes that introduce an indentation level: everything that opens a
; bracketed scope, plus binding-bearing expressions, call chains, and strings.
; The closing token is picked up by @outdent below.
[
  (attrset_expression)
  (rec_attrset_expression)
  (let_attrset_expression)
  (list_expression)
  (parenthesized_expression)
  (formals)
  (binding_set)
  (let_expression)
  (if_expression)
  (function_expression)
  (binary_expression)
  (apply_expression)
  (select_expression)
  (interpolation)
  (indented_string_expression)
  (string_expression)
] @indent

; Closing brackets end the indentation introduced above.
[
  "}"
  ")"
  "]"
] @outdent

; `let ... in ...` - the `in` clause is aligned with `let`.
(let_expression
  "in" @align)

; `if ... then ... else ...` - each keyword starts a branch at the same
; outer indent level.
(if_expression
  [
    "if"
    "then"
    "else"
  ] @align)
