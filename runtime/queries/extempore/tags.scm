; tags.scm --- Extempore code navigation (ctags-like)
; bind-func defines a function (name as symbol)
(list
  .
  (symbol) @_keyword
  .
  (symbol) @name
  (#any-of? @_keyword "bind-func" "definec" "definec:dsp" "bind-instrument" "bind-sampler")) @definition.function

; bind-func defines a function (name as typed_identifier)
(list
  .
  (symbol) @_keyword
  .
  (typed_identifier) @name
  (#any-of? @_keyword "bind-func" "definec" "definec:dsp" "bind-instrument" "bind-sampler")) @definition.function

; define with a list (function shorthand)
(list
  .
  (symbol) @_keyword
  .
  (list
    .
    (symbol) @name)
  (#eq? @_keyword "define")) @definition.function

; define with a symbol (variable)
(list
  .
  (symbol) @_keyword
  .
  (symbol) @name
  (#any-of? @_keyword "define" "bind-val")) @definition.var

; bind-type defines a type
(list
  .
  (symbol) @_keyword
  .
  (symbol) @name
  (#any-of? @_keyword "bind-type" "bind-data" "bind-alias")) @definition.type

; bind-data with generic_identifier
(list
  .
  (symbol) @_keyword
  .
  (generic_identifier) @name
  (#eq? @_keyword "bind-data")) @definition.type

; define-macro defines a macro
(list
  .
  (symbol) @_keyword
  .
  (symbol) @name
  (#any-of? @_keyword "define-macro" "bind-macro" "macro")) @definition.macro

(list
  .
  (symbol) @_keyword
  .
  (list
    .
    (symbol) @name)
  (#any-of? @_keyword "define-macro" "macro")) @definition.macro

; bind-poly defines a polymorphic binding
(list
  .
  (symbol) @_keyword
  .
  (symbol) @name
  (#eq? @_keyword "bind-poly")) @definition.function

; bind-lib defines a library binding
(list
  .
  (symbol) @_keyword
  .
  (symbol) @name
  (#any-of? @_keyword "bind-lib" "bind-dylib")) @definition.function

; Function calls
(list
  .
  (symbol) @name) @reference.call
