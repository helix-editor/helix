; indents.scm --- Extempore / xtlang indentation
;
; Follows `extempore fmt` (src/Formatter.cpp in the Extempore repo), which
; formats the Extempore source tree and is the reference for extempore-mode in
; Emacs as well. Its table of special forms is the one these lists copy.
;
; - a call aligns its arguments with the first one when that is on the head's
;   line, and indents them one step otherwise
; - data aligns with its first element: a list whose head is not a symbol (a
;   binding list, a cond clause, a list of numbers), a quoted list, a vector
; - a special form with N distinguished arguments (lambda has one) indents its
;   body one step, unless more than N arguments are on the head's line, in
;   which case the body aligns with the first of the rest
; - a definition form (bind-func, and after any namespace define, define*,
;   define-..., definec..., def-..., with-...) indents its body one step
;
; A call's step is @indent.always, which Helix counts inside an @align even on
; the line the alignment starts (`(foo (bar` puts bar's arguments two columns
; in from bar). An @indent there doesn't count, so a special form that starts
; mid-line aligns its body with its first form instead, and nothing gives a
; distinguished argument on a line of its own (do's test clause) the
; formatter's extra step. The formatter settles both on save.
; multi-line strings and block comments keep their indentation
[
  (string)
  (block_comment)
] @opaque

; --- calls ---
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
    "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
    "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
    "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
    "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
    "call-with-values" "let" "let*" "letrec" "letz" "syntax-case" "do" "receive" "dynamic-wind"
    "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val" "bind-lib" "bind-dylib" "bind-alias"
    "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument" "bind-sampler" "macro")
  (#not-match? @_head "^([^:]*:)*(define([*]|-.*|c.*)?|def-.*|with-.*)$")) @align

; a call whose head is alone on its line: one step
(list
  .
  [
    (symbol)
    (typed_identifier)
    (sharp_constant)
  ] @_head
  .
  (_) @_next
  (#not-same-line? @_head @_next)
  (#not-any-of? @_head
    "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
    "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
    "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
    "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
    "call-with-values" "let" "let*" "letrec" "letz" "syntax-case" "do" "receive" "dynamic-wind"
    "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val" "bind-lib" "bind-dylib" "bind-alias"
    "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument" "bind-sampler" "macro")
  (#not-match? @_head "^([^:]*:)*(define([*]|-.*|c.*)?|def-.*|with-.*)$")) @indent.always

(list
  .
  [
    (symbol)
    (typed_identifier)
    (sharp_constant)
  ] @_head
  .
  (comment)
  (#not-any-of? @_head
    "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
    "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
    "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
    "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
    "call-with-values" "let" "let*" "letrec" "letz" "syntax-case" "do" "receive" "dynamic-wind"
    "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val" "bind-lib" "bind-dylib" "bind-alias"
    "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument" "bind-sampler" "macro")
  (#not-match? @_head "^([^:]*:)*(define([*]|-.*|c.*)?|def-.*|with-.*)$")) @indent.always

(list
  .
  [
    (symbol)
    (typed_identifier)
    (sharp_constant)
  ] @_head
  .
  (#not-any-of? @_head
    "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
    "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
    "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
    "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
    "call-with-values" "let" "let*" "letrec" "letz" "syntax-case" "do" "receive" "dynamic-wind"
    "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val" "bind-lib" "bind-dylib" "bind-alias"
    "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument" "bind-sampler" "macro")
  (#not-match? @_head "^([^:]*:)*(define([*]|-.*|c.*)?|def-.*|with-.*)$")) @indent.always

; --- special forms: more arguments on the head's line than distinguished ---
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
    "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
    "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
    "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
    "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
    "call-with-values")) @align

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

; two: a named let, (let loop ((i 0)) ...), do, receive and syntax-case
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
  (#any-of? @_head "syntax-case" "do" "receive")) @align

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
  (#any-of? @_head "dynamic-wind")) @align

; --- special and definition forms: the body indents one step ---
(list
  .
  (symbol) @_head
  (#any-of? @_head
    "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
    "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
    "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
    "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
    "call-with-values" "let" "let*" "letrec" "letz" "syntax-case" "do" "receive" "dynamic-wind"
    "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val" "bind-lib" "bind-dylib" "bind-alias"
    "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument" "bind-sampler" "macro")) @indent

(list
  .
  (symbol) @_head
  (#match? @_head "^([^:]*:)*(define([*]|-.*|c.*)?|def-.*|with-.*)$")) @indent

; --- calls and special forms that start mid-line ---
;
; (let ((f (lambda (x)
;            (* x x))))
;
; Helix indents such a body from the enclosing alignment or the line's
; indentation, not from the form's own paren, so align it with its first form
; instead: reindenting leaves that line where it is, and the lines after it
; follow. The same goes for a list after a quote mark.
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
    (#not-any-of? @_head
      "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
      "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
      "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
      "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
      "call-with-values" "let" "let*" "letrec" "letz" "syntax-case" "do" "receive" "dynamic-wind"
      "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val" "bind-lib" "bind-dylib"
      "bind-alias" "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument" "bind-sampler"
      "macro")
    (#not-match? @_head "^([^:]*:)*(define([*]|-.*|c.*)?|def-.*|with-.*)$")) @align
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
    (#not-any-of? @_head
      "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
      "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
      "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
      "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
      "call-with-values" "let" "let*" "letrec" "letz" "syntax-case" "do" "receive" "dynamic-wind"
      "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val" "bind-lib" "bind-dylib"
      "bind-alias" "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument" "bind-sampler"
      "macro")
    (#not-match? @_head "^([^:]*:)*(define([*]|-.*|c.*)?|def-.*|with-.*)$")) @align)

(quasiquote
  (list
    .
    (symbol) @_head
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#not-any-of? @_head
      "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
      "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
      "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
      "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
      "call-with-values" "let" "let*" "letrec" "letz" "syntax-case" "do" "receive" "dynamic-wind"
      "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val" "bind-lib" "bind-dylib"
      "bind-alias" "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument" "bind-sampler"
      "macro")
    (#not-match? @_head "^([^:]*:)*(define([*]|-.*|c.*)?|def-.*|with-.*)$")) @align)

(quote
  (list
    .
    (symbol) @_head
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#not-any-of? @_head
      "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
      "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
      "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
      "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
      "call-with-values" "let" "let*" "letrec" "letz" "syntax-case" "do" "receive" "dynamic-wind"
      "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val" "bind-lib" "bind-dylib"
      "bind-alias" "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument" "bind-sampler"
      "macro")
    (#not-match? @_head "^([^:]*:)*(define([*]|-.*|c.*)?|def-.*|with-.*)$")) @align)

(unquote
  (list
    .
    (symbol) @_head
    .
    (_) @anchor
    (#not-same-line? @_head @anchor)
    (#not-kind-eq? @anchor "comment")
    (#not-any-of? @_head
      "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
      "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
      "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
      "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
      "call-with-values" "let" "let*" "letrec" "letz" "syntax-case" "do" "receive" "dynamic-wind"
      "bind-func" "bind-macro" "bind-poly" "bind-type" "bind-val" "bind-lib" "bind-dylib"
      "bind-alias" "bind-data" "bind-ext-val" "bind-lib-val" "bind-instrument" "bind-sampler"
      "macro")
    (#not-match? @_head "^([^:]*:)*(define([*]|-.*|c.*)?|def-.*|with-.*)$")) @align)

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
      "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
      "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
      "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
      "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
      "call-with-values")) @align
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
      "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
      "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
      "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
      "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
      "call-with-values")) @align)

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
      "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
      "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
      "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
      "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
      "call-with-values")) @align)

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
      "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
      "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
      "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
      "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
      "call-with-values")) @align)

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
      "lambda" "lambda*" "lambdaz" "lambdas" "lambdah" "case" "dotimes" "doloop" "while" "when"
      "unless" "memzone" "letrec*" "let-values" "let*-values" "parameterize" "guard" "let-syntax"
      "letrec-syntax" "syntax-rules" "call-with-input-file" "with-input-from-file"
      "with-input-from-port" "call-with-output-file" "with-output-to-file" "with-output-to-port"
      "call-with-values")) @align)

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

; --- data: a quoted list (and a list inside it) or a vector aligns with its
; first element. Last, so that it wins over the call patterns above.
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

(quote
  (list
    (list
      (list
        .
        (_) @anchor
        (#not-kind-eq? @anchor "comment")) @align)))

(vector
  .
  (_) @anchor
  (#not-kind-eq? @anchor "comment")) @align
