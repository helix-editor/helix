; Highlights queries from Matthew Fluet (https://github.com/MatthewFluet/tree-sitter-sml)
;
; MIT License
;
; Copyright (c) 2022 Matthew Fluet
;
; Permission is hereby granted, free of charge, to any person obtaining a copy
; of this software and associated documentation files (the "Software"), to deal
; in the Software without restriction, including without limitation the rights
; to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
; copies of the Software, and to permit persons to whom the Software is
; furnished to do so, subject to the following conditions:
;
; The above copyright notice and this permission notice shall be included in all
; copies or substantial portions of the Software.
;
; THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
; IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
; FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
; AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
; LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
; OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
; SOFTWARE.

;; *******************************************************************
;; Comments
;; *******************************************************************

[(block_comment) (line_comment)] @comment

;; *******************************************************************
;; Keywords
;; *******************************************************************

[
 ;; Reserved Words Core
 "abstype" "and" "andalso" "as" "case" "datatype" "do" "else" "end"
 "exception" "fn" "fun" "handle" "if" "in" "infix" "infixr" "let"
 "local" "nonfix" "of" "op" "open" "orelse" "raise" "rec" "then"
 "type" "val" "with" "withtype" "while"
 ;; Reserved Words Modules
 "eqtype" "functor" "include" "sharing" "sig" "signature" "struct"
 "structure" "where"
] @keyword

;; *******************************************************************
;; Constants
;; *******************************************************************

(integer_scon) @constant.numeric.integer
(real_scon) @constant.numeric.float
(word_scon) @constant.numeric
[(string_scon) (char_scon)] @string

;; *******************************************************************
;; Types
;; *******************************************************************

(fn_ty "->" @type)
(tuple_ty "*" @type)
(paren_ty ["(" ")"] @type)
(tyvar_ty (tyvar) @type)
(record_ty
 ["{" "," "}"] @type
 (tyrow [(lab) ":"] @type)?
 (ellipsis_tyrow ["..." ":"] @type)?)
(tycon_ty
 (tyseq ["(" "," ")"] @type)?
 (longtycon) @type)

;; *******************************************************************
;; Constructors
;; *******************************************************************

;; Assume value identifiers starting with capital letter are constructors
((vid) @constructor
 (#match? @constructor "^[A-Z].*"))
(longvid ((vid) @vid
          (#match? @vid "^[A-Z].*"))) @constructor

;; "true", "false", "nil", "::", and "ref" are built-in constructors
((vid) @constant.builtin
 (#match? @constant.builtin "true"))
((vid) @constant.builtin
 (#match? @constant.builtin "false"))
((vid) @constant.builtin
 (#match? @constant.builtin "nil"))
((vid) @constant.builtin
 (#match? @constant.builtin "::"))
((vid) @constant.builtin
 (#match? @constant.builtin "ref"))
(longvid ((vid) @vid
          (#match? @vid "true"))) @constant.builtin
(longvid ((vid) @vid
          (#match? @vid "false"))) @constant.builtin
(longvid ((vid) @vid
          (#match? @vid "nil"))) @constant.builtin
(longvid ((vid) @vid
           (#match? @vid "::"))) @constant.builtin
(longvid ((vid) @vid
          (#match? @vid "ref"))) @constant.builtin

;; *******************************************************************
;; Punctuation
;; *******************************************************************

["(" ")" "[" "]" "{" "}"] @punctuation.bracket
["." "," ":" ";" "|" "=>" ":>"] @punctuation.delimiter
