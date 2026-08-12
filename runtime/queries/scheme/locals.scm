; Everything in Scheme is `(list (symbol) ...)`, so scopes and definitions are
; gated by `#eq?`/`#any-of?` predicates on the leading symbol of a form.

; Scopes

(list
  .
  (symbol) @_f
  (#any-of? @_f
    "lambda" "λ" "case-lambda"
    "let" "let*" "letrec" "letrec*" "let-values" "let*-values"
    "let-syntax" "letrec-syntax"
    "do")) @local.scope

; A `(define (f args...) ...)` form is also a scope for its parameters.
(list
  .
  (symbol) @_f
  .
  (list)
  (#eq? @_f "define")) @local.scope

; Definitions

; (lambda (a b c) ...) and (define (f a b c) ...) parameters
(list
  .
  (symbol) @_f
  .
  (list
    (symbol) @local.definition.variable.parameter)
  (#any-of? @_f "lambda" "λ"))

(list
  .
  (symbol) @_f
  .
  (list
    .
    (symbol) ; function name, not a parameter
    (symbol) @local.definition.variable.parameter)
  (#eq? @_f "define"))

; (let ((x v) (y w)) ...) and friends: each binding's first symbol
(list
  .
  (symbol) @_f
  .
  (list
    (list
      .
      (symbol) @local.definition.variable))
  (#any-of? @_f
    "let" "let*" "letrec" "letrec*" "let-values" "let*-values"
    "let-syntax" "letrec-syntax" "do"))

; (define name value)
(list
  .
  (symbol) @_f
  .
  (symbol) @local.definition.variable
  (#eq? @_f "define"))

; References

(symbol) @local.reference

; The leading symbol of a form is a keyword/operator/call target, not a
; variable reference; cancel resolution there.
(list
  .
  (symbol) @_)
