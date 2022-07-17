(number) @constant.numeric
(character) @constant.character
(boolean) @constant.builtin.boolean

[(string)
 (character)] @string

(escape_sequence) @constant.character.escape

[(comment)
 (block_comment)
 (directive)] @comment

[(boolean)
 (character)] @constant

((symbol) @function.builtin
 (#match? @function.builtin "^(eqv\\?|eq\\?|equal\\?)")) ; TODO

; keywords

((symbol) @keyword.conditional
 (#match? @keyword.conditional "^(if|cond|case|when|unless)$"))
 
((symbol) @keyword
 (#match? @keyword
  "^(define|lambda|begin|do|define-syntax|and|or|if|cond|case|when|unless|else|=>|let|let*|let-syntax|let-values|let*-values|letrec|letrec*|letrec-syntax|set!|syntax-rules|identifier-syntax|quote|unquote|quote-splicing|quasiquote|unquote-splicing|delay|assert|library|export|import|rename|only|except|prefix)$"))

; special forms

(list
 "["
 (symbol)+ @variable
 "]")

(list
 .
 (symbol) @_f
 .
 (list
   (symbol) @variable)
 (#eq? @_f "lambda"))

(list
 .
 (symbol) @_f
 .
 (list
   (list
     (symbol) @variable))
 (#match? @_f
  "^(let|let\\*|let-syntax|let-values|let\\*-values|letrec|letrec\\*|letrec-syntax)$"))

; operators

(list
  .
  (symbol) @operator
  (#match? @operator "^([+*/<>=-]|(<=)|(>=))$"))
  
; quote

(abbreviation
  "'" (symbol)) @constant

(list
 .
 (symbol) @_f
 (#eq? @_f "quote")) @symbol

; library

(list
 .
 (symbol) @_lib
 .
 (symbol) @namespace

 (#eq? @_lib "library"))

; procedure

(list
  .
  (symbol) @function)

;; variables

((symbol) @variable.builtin
 (#eq? @variable.builtin "..."))

(symbol) @variable
((symbol) @variable.builtin
 (#eq? @variable.builtin "."))

(symbol) @variable


["(" ")" "[" "]" "{" "}"] @punctuation.bracket

