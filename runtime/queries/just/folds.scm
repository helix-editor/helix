; From <https://github.com/IndianBoy42/tree-sitter-just/blob/6c2f018ab1d90946c0ce029bb2f7d57f56895dff/queries-flavored/helix/folds.scm>

; Define collapse points

([
  (recipe)
  (string)
  (external_command)
] @fold
  (#trim! @fold))
