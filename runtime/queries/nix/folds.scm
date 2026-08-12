; Fold ranges for Nix.
;
; Nix has no syntactic block construct (no `{ ... }` statement blocks)
; - every region is an expression. The folds below cover the
; expression forms that are commonly multi-line in real Nix code.
;
; Capture set matches nvim-treesitter's vendored `runtime/queries/nix/
; folds.scm`, plus the additions noted at the bottom.
;
; Editors compute fold regions from each captured node's range.
; Single-line captures are harmless.
[
  (if_expression)
  (with_expression)
  (let_expression)
  (function_expression)
  (attrset_expression)
  (rec_attrset_expression)
  (list_expression)
  (indented_string_expression)
  (let_attrset_expression)
] @fold

; Additions over the nvim-treesitter reference set ---------------------

; Multi-line block / doc comments.
[
  (block_comment)
  (doc_comment)
] @fold
