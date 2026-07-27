; CLASS  top-level definition blocks
(defcfg)      @class.around @class.outer
(defsrc)      @class.around @class.outer
(deflayer)    @class.around @class.outer
(deflayermap) @class.around @class.outer
(defalias)    @class.around @class.outer
(defvar)      @class.around @class.outer
(deflocalkeys) @class.around @class.outer

; Inner part of a class: everything except the opening keyword
(defcfg
  key: (_) @class.inside @class.inner)
(defsrc
  keys: (_) @class.inside @class.inner)
(deflayer
  keys: (_) @class.inside @class.inner)
(deflayermap
  input: (_) @class.inside @class.inner)
(defalias
  name: (_) @class.inside @class.inner)
(defvar
  name: (_) @class.inside @class.inner)

; FUNCTION any parenthesised action list
(list) @function.around @function.outer

; Inner: the arguments, excluding the head
(list
  body: (_) @function.inside @function.inner)

(list
  body: (_) @parameter.inside @parameter.inner
             @parameter.around @parameter.outer)

(list
  head: (identifier) @_cond
  (#any-of? @_cond
    "switch" "fork"
    "if-equal" "if-not-equal" "if-in-list" "if-not-in-list")) @conditional.outer @conditional.around

(list
  head: (identifier) @_cond
  (#any-of? @_cond
    "switch" "fork"
    "if-equal" "if-not-equal" "if-in-list" "if-not-in-list")
  body: (_) @conditional.inner @conditional.inside)

(line_comment)  @comment.inside @comment.inner
(block_comment) @comment.inside @comment.inner
(line_comment)+  @comment.around @comment.outer
(block_comment)  @comment.around @comment.outer
