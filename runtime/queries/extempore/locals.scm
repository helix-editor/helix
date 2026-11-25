; locals.scm --- scopes, the names they bind, and references to them
;
; A reference to a name bound in an enclosing scope takes the highlight of its
; definition, so a lambda's parameters stay parameter-coloured in its body.
; The capture names follow Helix (@local.definition.variable.parameter); the
; tree-sitter CLI doesn't resolve these references.
; --- scopes ---
(list
  .
  (symbol) @_k
  (#any-of? @_k
    "lambda" "lambdaz" "lambdas" "lambdah" "let" "let*" "letrec" "letz" "do" "dotimes" "doloop"
    "memzone")) @local.scope

; (define (name param ...) body) scopes its parameters
(list
  .
  (symbol) @_k
  .
  (list)
  (#eq? @_k "define")) @local.scope

; --- definitions ---
(list
  .
  (symbol) @_k
  .
  (list
    [
      (symbol) @local.definition.variable.parameter
      (typed_identifier
        (symbol) @local.definition.variable.parameter)
    ])
  (#any-of? @_k "lambda" "lambdaz" "lambdas" "lambdah"))

(list
  .
  (symbol) @_k
  .
  (list
    .
    (symbol)
    (symbol) @local.definition.variable.parameter)
  (#eq? @_k "define"))

(list
  .
  (symbol) @_k
  .
  (list
    (list
      .
      [
        (symbol) @local.definition.variable
        (typed_identifier
          (symbol) @local.definition.variable)
      ]))
  (#any-of? @_k "let" "let*" "letrec" "letz"))

(list
  .
  (symbol) @_k
  .
  (list
    .
    [
      (symbol) @local.definition.variable
      (typed_identifier
        (symbol) @local.definition.variable)
    ])
  (#any-of? @_k "dotimes" "doloop"))

; (let loop ((name value) ...) body)
(list
  .
  (symbol) @_k
  .
  (symbol)
  .
  (list
    (list
      .
      [
        (symbol) @local.definition.variable
        (typed_identifier
          (symbol) @local.definition.variable)
      ]))
  (#eq? @_k "let"))

; (do ((i 0 (+ i 1))) ...)
(list
  .
  (symbol) @_k
  .
  (list
    (list
      .
      (symbol) @local.definition.variable))
  (#eq? @_k "do"))

; --- references ---
(symbol) @local.reference
