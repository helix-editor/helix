; textobjects.scm --- Helix textobjects
;
; A top-level form is the unit you evaluate, so it is the function textobject:
; `maf` selects the form at the cursor and `]f`/`[f` step between forms.
(program
  (list) @function.around)

; the body of a lambda or a (define (name ...) ...)
(list
  .
  (symbol) @_k
  .
  (list)
  .
  (_)+ @function.inside
  (#any-of? @_k "lambda" "define" "define-macro"))

(list
  .
  (symbol) @_k
  (#any-of? @_k "bind-type" "bind-data" "bind-alias")) @class.around

(list
  .
  (symbol) @_k
  (#match? @_k "^xtmtest")) @test.around

; parameters of a lambda, and of a (define (name param ...) ...)
(list
  .
  (symbol) @_k
  .
  (list
    (_) @parameter.inside @parameter.around)
  (#eq? @_k "lambda"))

(list
  .
  (symbol) @_k
  .
  (list
    .
    (_)
    (_) @parameter.inside @parameter.around)
  (#eq? @_k "define"))

; the elements of a quoted list or a vector
(quote
  (list
    (_) @entry.inside @entry.around))

(vector
  (_) @entry.inside @entry.around)

[
  (comment)
  (block_comment)
] @comment.inside

(comment)+ @comment.around

(block_comment) @comment.around
