; Opening a parenthesis always starts an indented block.
[
  (defcfg)
  (defsrc)
  (deflayer)
  (deflayermap)
  (defalias)
  (defvar)
  (deflocalkeys)
  (list)
] @indent @indent.begin

; The closing ")" reduces indentation by one level.
")" @outdent @indent.branch
