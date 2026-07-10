; Top-level definition blocks as classes (deflayer, defalias, defcfg, ...)
(list
  . (identifier) @_def
  (#any-of? @_def
    "defcfg" "defsrc" "deflayer" "deflayer-mapped" "deflayermap"
    "defalias" "defaliasenvcond" "defvar" "deftemplate"
    "deffakekeys" "defvirtualkeys" "defchords" "defchordsv2"
    "defchordsv2-experimental" "defzippy" "defzippy-experimental"
    "defseq" "defhands" "definputdevices" "defoverrides" "defoverridesv2"
    "deflocalkeys-macos" "deflocalkeys-linux" "deflocalkeys-win"
    "deflocalkeys-winiov2" "deflocalkeys-wintercept"
    "platform" "environment")
  (_)* @class.inside) @class.around

; Any list form as a function-like block
(list
  (_)* @function.inside) @function.around

; Elements within a list as parameters
(list
  (_) @parameter.inside @parameter.around)

; Comments
(line_comment) @comment.inside
(block_comment) @comment.inside
(line_comment)+ @comment.around
(block_comment) @comment.around

