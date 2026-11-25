; highlights.scm --- Extempore syntax highlighting
;
; Capture names are Helix's theme scopes (https://docs.helix-editor.com/themes.html);
; .tsqueryrc.json lists the ones allowed. Patterns run from general to
; specific: where two patterns capture the same node, the later one wins.
; Everything in Extempore is `(list (symbol) ...)`, so forms are told apart by
; `#any-of?` on the leading symbol.
; --- general ---
; the leading symbol of a form is a call
(list
  .
  (symbol) @function)

(number) @constant.numeric

(string) @string

(escape_sequence) @constant.character.escape

(boolean) @constant.builtin.boolean

(character) @constant.character

(sharp_constant) @constant.builtin

((symbol) @constant.builtin
  (#eq? @constant.builtin "null"))

(comment) @comment.line

(block_comment) @comment.block

[
  "("
  ")"
  "#("
] @punctuation.bracket

(list
  "." @punctuation.delimiter)

(quote
  "'" @punctuation.special)

(quasiquote
  "`" @punctuation.special)

(unquote
  "," @punctuation.special)

(unquote_splicing
  ",@" @punctuation.special)

; 'name and '(a b c): data, not calls
(quote
  (symbol) @string.special.symbol)

(quote
  (list
    (symbol) @string.special.symbol))

; --- xtlang types ---
(xtlang_type) @type

(type_annotation) @type

(generic_identifier) @type

((symbol) @type.builtin
  (#any-of? @type.builtin "i1" "i8" "i16" "i32" "i64" "f" "f32" "f64" "d" "float" "double" "void"))

; --- calls to known names ---
(list
  .
  (symbol) @operator
  (#any-of? @operator
    "+" "-" "*" "/" "=" "<" ">" "<=" ">=" "!=" "%" "<<" ">>" "bitwise-and" "bitwise-or"
    "bitwise-xor" "bitwise-not"))

(list
  .
  (symbol) @keyword.operator
  (#any-of? @keyword.operator "$" "$$"))

; Scheme
(list
  .
  (symbol) @function.builtin
  (#any-of? @function.builtin
    "car" "cdr" "cons" "list" "append" "reverse" "length" "map" "for-each" "filter" "fold" "foldl"
    "foldr" "apply" "eval" "null?" "pair?" "number?" "integer?" "real?" "string?" "symbol?"
    "procedure?" "boolean?" "char?" "vector?" "port?" "eq?" "eqv?" "equal?" "not" "zero?"
    "positive?" "negative?" "even?" "odd?" "abs" "min" "max" "modulo" "remainder" "quotient" "gcd"
    "lcm" "floor" "ceiling" "truncate" "round" "exact->inexact" "inexact->exact" "number->string"
    "string->number" "string->symbol" "symbol->string" "string->list" "list->string" "vector->list"
    "list->vector" "char->integer" "integer->char" "string-ref" "string-set!" "string-length"
    "string-append" "string-copy" "substring" "string-upcase" "string-downcase" "make-string"
    "make-vector" "vector-ref" "vector-set!" "vector-length" "vector-fill!" "assoc" "assq" "assv"
    "member" "memq" "memv" "sort" "display" "write" "newline" "println" "print" "printf" "printout"
    "read" "error" "format" "open-input-file" "open-output-file" "close-input-port"
    "close-output-port" "read-char" "write-char" "peek-char" "char-ready?" "exit" "quit" "caar"
    "cadr" "cdar" "cddr" "caaar" "caadr" "cadar" "caddr" "cdaar" "cdadr" "cddar" "cdddr"))

; xtlang memory and data
(list
  .
  (symbol) @function.builtin
  (#any-of? @function.builtin
    "alloc" "salloc" "halloc" "zalloc" "stack-alloc" "heap-alloc" "zone-alloc" "free" "tref" "tset!"
    "tfill!" "aref" "aset!" "aref-ptr" "afill!" "pref" "pset!" "pref-ptr" "pfill!" "vref" "vset!"
    "vfill!" "make-array" "array-ref" "array-set!" "type-of" "size-of" "now" "random" "imp_rand"))

; --- keywords ---
(list
  .
  (symbol) @keyword
  (#any-of? @keyword
    "define" "define-macro" "macro" "lambda" "let" "let*" "letrec" "letz" "begin" "beginz" "set!"
    "and" "or" "else" "delay" "receive" "quote" "quasiquote" "unquote" "unquote-splicing" "memzone"
    "callback" "cast" "convert" "bitcast" "call-as-xtlang" "sync" "spawn" "async" "catch" "call/cc"
    "call-with-current-continuation"))

(list
  .
  (symbol) @keyword
  (#any-of? @keyword
    "bind-func" "bind-type" "bind-data" "bind-val" "bind-ext-val" "bind-lib" "bind-dylib"
    "bind-alias" "bind-poly" "bind-macro" "bind-instrument" "bind-sampler" "definec" "definec:dsp"))

(list
  .
  (symbol) @keyword.control.conditional
  (#any-of? @keyword.control.conditional "if" "cond" "case" "when" "unless"))

(list
  .
  (symbol) @keyword.control.repeat
  (#any-of? @keyword.control.repeat "do" "dotimes" "doloop" "dolet" "while"))

(list
  .
  (symbol) @keyword.control.import
  (#any-of? @keyword.control.import "sys:load" "load"))

; --- the names a form binds ---
; (define name value)
(list
  .
  (symbol) @keyword
  .
  (symbol) @variable
  (#eq? @keyword "define"))

; (define (name param ...) "doc" body)
(list
  .
  (symbol) @keyword
  .
  (list
    .
    (symbol) @function
    (symbol)* @variable.parameter)
  .
  (string)? @comment.block.documentation
  .
  (_)
  (#eq? @keyword "define"))

; (define-macro (name param ...) body), (macro (name ...) body)
(list
  .
  (symbol) @keyword
  .
  (list
    .
    (symbol) @function.macro)
  (#any-of? @keyword "define-macro" "macro"))

; (bind-macro "doc" (name . args) body)
(list
  .
  (symbol) @keyword
  .
  (string)? @comment.block.documentation
  .
  (list
    .
    (symbol) @function.macro)
  (#eq? @keyword "bind-macro"))

; (bind-func name "doc" (lambda ...)); the name may carry a type, name:[ret,args]*
(list
  .
  (symbol) @keyword
  .
  [
    (symbol) @function
    (typed_identifier
      (symbol) @function)
  ]
  .
  (string)? @comment.block.documentation
  .
  (_)
  (#any-of? @keyword "bind-func" "definec" "definec:dsp" "bind-instrument" "bind-sampler"))

; (bind-type Name <...>), (bind-alias Name type), (bind-data Name{!a} ...)
(list
  .
  (symbol) @keyword
  .
  [
    (symbol)
    (generic_identifier)
  ] @type
  (#any-of? @keyword "bind-type" "bind-alias" "bind-data"))

; (bind-val NAME type value "doc")
(list
  .
  (symbol) @keyword
  .
  (symbol) @variable
  (#any-of? @keyword "bind-val" "bind-ext-val"))

; (bind-poly name impl)
(list
  .
  (symbol) @keyword
  .
  (symbol) @function
  .
  (symbol) @function
  (#eq? @keyword "bind-poly"))

; (bind-lib lib fn type), (bind-dylib lib "path")
(list
  .
  (symbol) @keyword
  .
  (symbol) @namespace
  .
  (symbol)? @function
  (#any-of? @keyword "bind-lib" "bind-dylib"))

; (lambda (param ...) body); params may carry a type, x:i64. One match captures
; every parameter: with a match per parameter, the tree-sitter CLI lets the call
; pattern above win for the first one.
(list
  .
  (symbol) @keyword
  .
  (list
    [
      (symbol) @variable.parameter
      (typed_identifier
        (symbol) @variable.parameter)
    ]*)
  (#eq? @keyword "lambda"))

; (let ((name value) ...) body), as one match like the lambda pattern
(list
  .
  (symbol) @keyword
  .
  (list
    (list
      .
      [
        (symbol) @variable
        (typed_identifier
          (symbol) @variable)
      ])*)
  (#any-of? @keyword "let" "let*" "letrec" "letz"))

; (dotimes (i n) body), (doloop (i n) body)
(list
  .
  (symbol) @keyword.control.repeat
  .
  (list
    .
    [
      (symbol) @variable
      (typed_identifier
        (symbol) @variable)
    ])
  (#any-of? @keyword.control.repeat "dotimes" "doloop"))
