; highlights.scm --- Extempore syntax highlighting
;
; Capture names are Helix's theme scopes (https://docs.helix-editor.com/themes.html);
; .tsqueryrc.json lists the ones allowed. Patterns run from general to
; specific: where two patterns capture the same node, the later one wins.
; Everything in Extempore is `(list (symbol) ...)`, so forms are told apart by
; `#any-of?` on the leading symbol. The xtlang word lists follow the compiler
; (runtime/xtc-caches.xtm and xtc-transforms.xtm in the Extempore repo).
; --- general ---
; the leading symbol of a form is a call
(list
  .
  (symbol) @function)

; (Rat 1 2), (Point 0.0 1.0): xtlang type constructors are capitalised
(list
  .
  (symbol) @constructor
  (#match? @constructor "^[A-Z][A-Za-z0-9]*$"))

(number) @constant.numeric

(string) @string

(escape_sequence) @constant.character.escape

(boolean) @constant.builtin.boolean

(character) @constant.character

(sharp_constant) @constant.builtin

((symbol) @constant.builtin
  (#eq? @constant.builtin "null"))

; SAMPLE_RATE, PI: globals by convention
((symbol) @constant
  (#match? @constant "^[A-Z][A-Z0-9_]+$"))

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

; --- calls to known names ---
(list
  .
  (symbol) @operator
  (#any-of? @operator
    "+" "-" "*" "/" "%" "=" "<" ">" "<=" ">=" "<>" "!=" "<<" ">>" "&" "bor" "^" "~" "bitwise-and"
    "bitwise-or" "bitwise-eor" "bitwise-xor" "bitwise-not" "bitwise-shift-left"
    "bitwise-shift-right"))

(list
  .
  (symbol) @keyword.operator
  (#any-of? @keyword.operator "and" "or" "not" "$" "$$"))

; Scheme
(list
  .
  (symbol) @function.builtin
  (#any-of? @function.builtin
    "car" "cdr" "cons" "list" "append" "reverse" "length" "list-ref" "list-tail" "last-pair" "map"
    "for-each" "filter" "fold" "foldl" "foldr" "reduce" "apply" "eval" "values" "call-with-values"
    "dynamic-wind" "null?" "pair?" "list?" "number?" "integer?" "rational?" "real?" "string?"
    "symbol?" "procedure?" "boolean?" "char?" "vector?" "port?" "hash-table?" "defined?" "eq?"
    "eqv?" "equal?" "zero?" "positive?" "negative?" "even?" "odd?" "exact?" "inexact?" "abs" "min"
    "max" "modulo" "remainder" "quotient" "gcd" "lcm" "floor" "ceiling" "truncate" "round"
    "exact->inexact" "inexact->exact" "real->integer" "number->string" "string->number"
    "string->symbol" "symbol->string" "string->list" "list->string" "vector->list" "list->vector"
    "char->integer" "integer->char" "string-ref" "string-set!" "string-length" "string-append"
    "string-copy" "substring" "string-upcase" "string-downcase" "string=?" "string<?" "string>?"
    "char=?" "char<?" "char>?" "make-string" "make-vector" "vector" "vector-ref" "vector-set!"
    "vector-length" "vector-fill!" "make-hash-table" "hash-table-ref" "hash-table-set!" "assoc"
    "assq" "assv" "member" "memq" "memv" "sort" "sort!" "set-car!" "set-cdr!" "display" "write"
    "newline" "print" "printout" "read" "error" "format" "open-input-file" "open-output-file"
    "close-input-port" "close-output-port" "read-char" "write-char" "peek-char" "char-ready?" "exit"
    "quit" "caar" "cadr" "cdar" "cddr" "caaar" "caadr" "cadar" "caddr" "cdaar" "cdadr" "cddar"
    "cdddr" "cadddr" "cddddr"))

; xtlang: memory, data and printing
(list
  .
  (symbol) @function.builtin
  (#any-of? @function.builtin
    "alloc" "salloc" "halloc" "zalloc" "stack-alloc" "heap-alloc" "zone-alloc" "free" "tref"
    "tref-ptr" "tset!" "tfill!" "aref" "aref-ptr" "aset!" "afill!" "pref" "pref-ptr" "pset!"
    "pfill!" "vref" "vref-ptr" "vset!" "vfill!" "vshuffle" "cref" "cset!" "refcheck" "tuple" "array"
    "make-array" "array-ref" "array-set!" "tuple-ref" "tuple-set!" "pointer-ref" "pointer-set!"
    "type-of" "size-of" "now" "random" "randomf" "println" "printf" "sprintf" "fprintf" "sscanf"
    "fscanf" "sprintln" "sprintout" "strln" "strj" "cat" "imp_rand"))

; maths, in Scheme and xtlang
(list
  .
  (symbol) @function.builtin
  (#any-of? @function.builtin
    "sin" "cos" "tan" "asin" "acos" "atan" "atan2" "sinh" "cosh" "tanh" "sqrt" "cbrt" "pow" "exp"
    "exp2" "expt" "log" "log2" "log10" "fabs" "ceil" "trunc" "fmod" "fmin" "fmax" "fma" "copysign"))

; xtlang numeric conversions: ftod, i64tof, dtoi64, ...
(list
  .
  (symbol) @function.builtin
  (#match? @function.builtin
    "^(i1|i8|i16|i32|i64|ui8|ui16|ui32|ui64|f|d)to(i1|i8|i16|i32|i64|ui8|ui16|ui32|ui64|f|d)$"))

; Extempore's test harness
(list
  .
  (symbol) @function.macro
  (#match? @function.macro "^xtmtest-"))

; --- keywords ---
(list
  .
  (symbol) @keyword
  (#any-of? @keyword
    "define" "define-macro" "macro" "define-syntax" "lambda" "lambdaz" "lambdas" "lambdah" "let"
    "let*" "letrec" "letz" "begin" "beginz" "set!" "delay" "receive" "quote" "quasiquote" "unquote"
    "unquote-splicing" "memzone" "zone_cleanup" "callback" "schedule" "cast" "convert" "bitcast"
    "call-as-xtlang" "sync" "spawn" "async" "catch" "call/cc" "call-with-current-continuation"))

(list
  .
  (symbol) @keyword
  (#any-of? @keyword
    "bind-func" "bind-type" "bind-data" "bind-val" "bind-ext-val" "bind-lib" "bind-lib-val"
    "bind-dylib" "bind-alias" "bind-poly" "bind-macro" "bind-instrument" "bind-sampler" "definec"
    "definec:dsp"))

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
  (#any-of? @keyword.control.import "sys:load" "sys:load-preload-check" "load"))

; else as the head of a cond or case clause
(list
  .
  (symbol) @keyword
  (#eq? @keyword "else"))

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
    (symbol) @function.macro
    (symbol)* @variable.parameter)
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
    (symbol) @function.macro
    (symbol)* @variable.parameter)
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

; (bind-func static name "doc" (lambda ...))
(list
  .
  (symbol) @keyword
  .
  (symbol) @keyword.storage.modifier
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
  (#eq? @keyword "bind-func")
  (#eq? @keyword.storage.modifier "static"))

; (bind-type Name <...> "doc"), (bind-alias Name type), (bind-data Name{!a} ...)
(list
  .
  (symbol) @keyword
  .
  [
    (symbol)
    (generic_identifier)
  ] @type
  .
  [
    (xtlang_type)
    (symbol)
  ]? @type
  (#any-of? @keyword "bind-type" "bind-alias" "bind-data"))

; (bind-val name type value "doc"); an all-caps NAME is a constant, above
(list
  .
  (symbol) @keyword
  .
  (symbol) @variable
  .
  [
    (xtlang_type)
    (symbol)
  ]? @type
  (#any-of? @keyword "bind-val" "bind-ext-val" "bind-lib-val")
  (#not-match? @variable "^[A-Z][A-Z0-9_]+$"))

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

; docstrings: (bind-val name type value "doc"), (bind-ext-val name type "doc").
; Each also captures the name, which the tree-sitter CLI otherwise leaves bare.
(list
  .
  (symbol) @keyword
  .
  (symbol) @variable
  .
  (_)
  .
  (_)
  .
  (string) @comment.block.documentation
  .
  (#any-of? @keyword "bind-val" "bind-lib-val")
  (#not-match? @variable "^[A-Z][A-Z0-9_]+$"))

(list
  .
  (symbol) @keyword
  .
  (symbol) @constant
  .
  (_)
  .
  (_)
  .
  (string) @comment.block.documentation
  .
  (#any-of? @keyword "bind-val" "bind-lib-val")
  (#match? @constant "^[A-Z][A-Z0-9_]+$"))

(list
  .
  (symbol) @keyword
  .
  (symbol) @variable
  .
  (_)
  .
  (string) @comment.block.documentation
  .
  (#eq? @keyword "bind-ext-val")
  (#not-match? @variable "^[A-Z][A-Z0-9_]+$"))

(list
  .
  (symbol) @keyword
  .
  (symbol) @constant
  .
  (_)
  .
  (string) @comment.block.documentation
  .
  (#eq? @keyword "bind-ext-val")
  (#match? @constant "^[A-Z][A-Z0-9_]+$"))

; (case key ((datum ...) expr) ...): the datums are data, one match per clause
(list
  .
  (symbol) @keyword.control.conditional
  .
  (_)
  (list
    .
    (list
      (symbol)* @string.special.symbol))
  (#eq? @keyword.control.conditional "case"))

; (convert x i64), (cast p Point*): the type argument
(list
  .
  (symbol) @keyword
  .
  (_)
  .
  [
    (symbol)
    (xtlang_type)
  ] @type
  (#any-of? @keyword "cast" "convert" "bitcast"))

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
  (#any-of? @keyword "lambda" "lambdaz" "lambdas" "lambdah"))

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

; (let loop ((name value) ...) body): a named let
(list
  .
  (symbol) @keyword
  .
  (symbol) @function
  .
  (list
    (list
      .
      [
        (symbol) @variable
        (typed_identifier
          (symbol) @variable)
      ])*)
  (#eq? @keyword "let"))

; (do ((i 0 (+ i 1))) ((= i n)) body)
(list
  .
  (symbol) @keyword.control.repeat
  .
  (list
    (list
      .
      (symbol) @variable)*)
  (#eq? @keyword.control.repeat "do"))

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

; builtin type names, wherever a pattern above calls them a type
((symbol) @type.builtin
  (#any-of? @type.builtin
    "i1" "i8" "i16" "i32" "i64" "ui8" "ui16" "ui32" "ui64" "f32" "f64" "float" "double" "void"))
