; From <https://github.com/IndianBoy42/tree-sitter-just/blob/6c2f018ab1d90946c0ce029bb2f7d57f56895dff/queries-flavored/helix/indents.scm>
;
; This query specifies how to auto-indent logical blocks.
;
; Better documentation with diagrams is in https://docs.helix-editor.com/guides/indent.html

[
  (recipe)
  (string)
  (external_command)
] @indent @extend
