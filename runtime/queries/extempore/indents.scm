; indents.scm --- Extempore / xtlang indentation
;
; Follows `extempore fmt` (src/Formatter.cpp in the Extempore repo), which
; formats the Extempore source tree and is the reference for extempore-mode in
; Emacs as well. Its table of special forms is the one these lists copy.
;
; - a call aligns its arguments with the first one when that is on the head's
;   line, and with the head otherwise
; - data aligns with its first element: a list whose head is not a symbol (a
;   binding list, a cond clause, a list of numbers), a quoted list, a vector
; - a special form with N distinguished arguments (lambda has one, begin none)
;   indents its body one step, unless more than N arguments are on the head's
;   line, in which case the body aligns with the first of the rest
; - a definition form (bind-func, define, define-..., definec) indents its
;   body one step
;
; Helix's engine counts an @indent only on a line other than the start of the
; innermost @align, so a special form that starts mid-line (`(let ((f (lambda
; (x)`) aligns its body with its first form rather than indenting from its own
; paren, and nothing gives a distinguished argument on a line of its own (do's
; test clause) the formatter's extra step. The formatter settles both on save.
; multi-line strings and block comments keep their indentation
[
  (string)
  (block_comment)
] @opaque

; --- calls and data ---
; the head is not a symbol: align with it
(list
  .
  (_) @anchor
  (#not-kind-eq? @anchor "symbol")
  (#not-kind-eq? @anchor "typed_identifier")
  (#not-kind-eq? @anchor "sharp_constant")) @align

; a call with an argument on the head's line: align with that argument
(list
  .
  [
    (symbol)
    (typed_identifier)
    (sharp_constant)
  ] @_head
  .
  (_) @anchor
  (#same-line? @_head @anchor)
  (#not-kind-eq? @anchor "comment")
  (#not-any-of? @_head
    "begin" "beginz" "delay" "lambda" "lambdaz" "lambdas" "lambdah" "let" "let*" "letrec" "letz"
    "case" "dotimes" "doloop" "while" "when" "unless" "memzone" "let-syntax" "letrec-syntax"
    "syntax-rules" "syntax-case" "do" "call-with-input-file" "with-input-from-file"
    "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
    "call-with-values" "dynamic-wind" "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val"
    "bind-lib" "bind-dylib" "bind-alias" "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument"
    "bind-sampler")
  (#not-match? @_head "^define(-.*|c.*)?$")) @align

; a call whose head is alone on its line: align with the head
(list
  .
  [
    (symbol)
    (typed_identifier)
    (sharp_constant)
  ] @anchor
  .
  (_) @_next
  (#not-same-line? @anchor @_next)
  (#not-any-of? @anchor
    "begin" "beginz" "delay" "lambda" "lambdaz" "lambdas" "lambdah" "let" "let*" "letrec" "letz"
    "case" "dotimes" "doloop" "while" "when" "unless" "memzone" "let-syntax" "letrec-syntax"
    "syntax-rules" "syntax-case" "do" "call-with-input-file" "with-input-from-file"
    "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
    "call-with-values" "dynamic-wind" "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val"
    "bind-lib" "bind-dylib" "bind-alias" "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument"
    "bind-sampler")
  (#not-match? @anchor "^define(-.*|c.*)?$")) @align

(list
  .
  [
    (symbol)
    (typed_identifier)
    (sharp_constant)
  ] @anchor
  .
  (comment)
  (#not-any-of? @anchor
    "begin" "beginz" "delay" "lambda" "lambdaz" "lambdas" "lambdah" "let" "let*" "letrec" "letz"
    "case" "dotimes" "doloop" "while" "when" "unless" "memzone" "let-syntax" "letrec-syntax"
    "syntax-rules" "syntax-case" "do" "call-with-input-file" "with-input-from-file"
    "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
    "call-with-values" "dynamic-wind" "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val"
    "bind-lib" "bind-dylib" "bind-alias" "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument"
    "bind-sampler")
  (#not-match? @anchor "^define(-.*|c.*)?$")) @align

(list
  .
  [
    (symbol)
    (typed_identifier)
    (sharp_constant)
  ] @anchor
  .
  (#not-any-of? @anchor
    "begin" "beginz" "delay" "lambda" "lambdaz" "lambdas" "lambdah" "let" "let*" "letrec" "letz"
    "case" "dotimes" "doloop" "while" "when" "unless" "memzone" "let-syntax" "letrec-syntax"
    "syntax-rules" "syntax-case" "do" "call-with-input-file" "with-input-from-file"
    "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
    "call-with-values" "dynamic-wind" "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val"
    "bind-lib" "bind-dylib" "bind-alias" "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument"
    "bind-sampler")
  (#not-match? @anchor "^define(-.*|c.*)?$")) @align

; data: a quoted list (and one directly inside it) or a vector aligns with its
; first element
(quote
  (list
    .
    (_) @anchor
    (#not-kind-eq? @anchor "comment")) @align)

(quote
  (list
    (list
      .
      (_) @anchor
      (#not-kind-eq? @anchor "comment")) @align))

(vector
  .
  (_) @anchor
  (#not-kind-eq? @anchor "comment")) @align

; --- special forms: more arguments on the head's line than distinguished ---
; no distinguished argument: (begin (a)
;                                   (b))
(list
  .
  (symbol) @_head
  .
  (_) @anchor
  (#same-line? @_head @anchor)
  (#not-kind-eq? @anchor "comment")
  (#any-of? @_head "begin" "beginz" "delay")) @align

; one: (lambda (x) (a)
;                  (b))
(list
  .
  (symbol) @_head
  .
  (_)
  .
  (_) @anchor
  (#same-line? @_head @anchor)
  (#not-kind-eq? @anchor "comment")
  (#any-of? @_head
    "lambda" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when" "unless"
    "memzone" "let-syntax" "letrec-syntax" "syntax-rules" "call-with-input-file"
    "with-input-from-file" "with-input-from-port" "call-with-output-file" "with-output-to-file"
    "with-output-to-port" "call-with-values")) @align

; let's bindings are its one distinguished argument
(list
  .
  (symbol) @_head
  .
  (list)
  .
  (_) @anchor
  (#same-line? @_head @anchor)
  (#not-kind-eq? @anchor "comment")
  (#any-of? @_head "let" "let*" "letrec" "letz")) @align

; two: a named let, (let loop ((i 0)) ...), and syntax-case
(list
  .
  (symbol) @_head
  .
  (symbol)
  .
  (_)
  .
  (_) @anchor
  (#same-line? @_head @anchor)
  (#not-kind-eq? @anchor "comment")
  (#any-of? @_head "let" "let*" "letrec" "letz")) @align

(list
  .
  (symbol) @_head
  .
  (_)
  .
  (_)
  .
  (_) @anchor
  (#same-line? @_head @anchor)
  (#not-kind-eq? @anchor "comment")
  (#any-of? @_head "syntax-case" "do")) @align

; three: dynamic-wind
(list
  .
  (symbol) @_head
  .
  (_)
  .
  (_)
  .
  (_)
  .
  (_) @anchor
  (#same-line? @_head @anchor)
  (#not-kind-eq? @anchor "comment")
  (#eq? @_head "dynamic-wind")) @align

; --- special and definition forms: the body indents one step ---
(list
  .
  (symbol) @_head
  (#any-of? @_head
    "begin" "beginz" "delay" "lambda" "lambdaz" "lambdas" "lambdah" "let" "let*" "letrec" "letz"
    "case" "dotimes" "doloop" "while" "when" "unless" "memzone" "let-syntax" "letrec-syntax"
    "syntax-rules" "syntax-case" "do" "call-with-input-file" "with-input-from-file"
    "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
    "call-with-values" "dynamic-wind" "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val"
    "bind-lib" "bind-dylib" "bind-alias" "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument"
    "bind-sampler")) @indent

(list
  .
  (symbol) @_head
  (#match? @_head "^define(-.*|c.*)?$")) @indent

; --- special forms that start mid-line ---
;
; (let ((f (lambda (x)
;            (* x x))))
;
; Helix indents such a body from the enclosing alignment, not from the form's
; own paren, so align it with its first form instead: reindenting leaves that
; line where it is, and the lines after it follow.
(_
  (_) @_prev
  .
  (list
    .
    (symbol) @_head
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "begin" "beginz" "delay")) @align
  (#same-line? @_prev @_head))

(list
  .
  (list
    .
    (symbol) @_head
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "begin" "beginz" "delay")) @align)

(quasiquote
  (list
    .
    (symbol) @_head
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "begin" "beginz" "delay")) @align)

(quote
  (list
    .
    (symbol) @_head
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "begin" "beginz" "delay")) @align)

(unquote
  (list
    .
    (symbol) @_head
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "begin" "beginz" "delay")) @align)

(_
  (_) @_prev
  .
  (list
    .
    (symbol) @_head
    .
    (_)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head
      "lambda" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when" "unless"
      "memzone" "let-syntax" "letrec-syntax" "syntax-rules" "call-with-input-file"
      "with-input-from-file" "with-input-from-port" "call-with-output-file" "with-output-to-file"
      "with-output-to-port" "call-with-values")) @align
  (#same-line? @_prev @_head))

(list
  .
  (list
    .
    (symbol) @_head
    .
    (_)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head
      "lambda" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when" "unless"
      "memzone" "let-syntax" "letrec-syntax" "syntax-rules" "call-with-input-file"
      "with-input-from-file" "with-input-from-port" "call-with-output-file" "with-output-to-file"
      "with-output-to-port" "call-with-values")) @align)

(quasiquote
  (list
    .
    (symbol) @_head
    .
    (_)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head
      "lambda" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when" "unless"
      "memzone" "let-syntax" "letrec-syntax" "syntax-rules" "call-with-input-file"
      "with-input-from-file" "with-input-from-port" "call-with-output-file" "with-output-to-file"
      "with-output-to-port" "call-with-values")) @align)

(quote
  (list
    .
    (symbol) @_head
    .
    (_)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head
      "lambda" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when" "unless"
      "memzone" "let-syntax" "letrec-syntax" "syntax-rules" "call-with-input-file"
      "with-input-from-file" "with-input-from-port" "call-with-output-file" "with-output-to-file"
      "with-output-to-port" "call-with-values")) @align)

(unquote
  (list
    .
    (symbol) @_head
    .
    (_)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head
      "lambda" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when" "unless"
      "memzone" "let-syntax" "letrec-syntax" "syntax-rules" "call-with-input-file"
      "with-input-from-file" "with-input-from-port" "call-with-output-file" "with-output-to-file"
      "with-output-to-port" "call-with-values")) @align)

(_
  (_) @_prev
  .
  (list
    .
    (symbol) @_head
    .
    (list)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "let" "let*" "letrec" "letz")) @align
  (#same-line? @_prev @_head))

(list
  .
  (list
    .
    (symbol) @_head
    .
    (list)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "let" "let*" "letrec" "letz")) @align)

(quasiquote
  (list
    .
    (symbol) @_head
    .
    (list)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "let" "let*" "letrec" "letz")) @align)

(quote
  (list
    .
    (symbol) @_head
    .
    (list)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "let" "let*" "letrec" "letz")) @align)

(unquote
  (list
    .
    (symbol) @_head
    .
    (list)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "let" "let*" "letrec" "letz")) @align)

(_
  (_) @_prev
  .
  (list
    .
    (symbol) @_head
    .
    (symbol)
    .
    (_)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "let" "let*" "letrec" "letz")) @align
  (#same-line? @_prev @_head))

(list
  .
  (list
    .
    (symbol) @_head
    .
    (symbol)
    .
    (_)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "let" "let*" "letrec" "letz")) @align)

(quasiquote
  (list
    .
    (symbol) @_head
    .
    (symbol)
    .
    (_)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "let" "let*" "letrec" "letz")) @align)

(quote
  (list
    .
    (symbol) @_head
    .
    (symbol)
    .
    (_)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "let" "let*" "letrec" "letz")) @align)

(unquote
  (list
    .
    (symbol) @_head
    .
    (symbol)
    .
    (_)
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#any-of? @_head "let" "let*" "letrec" "letz")) @align)
