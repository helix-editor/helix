;; Constants
(integer) @constant.numeric.integer
; (float) @constant.numeric.float
(literal) @string

(macro) @function.macro

;; Pragmas and comments
(pragma) @attribute
(comment) @comment

;; Imports
(open) @keyword.control.import
(module) @keyword.control.import
(module_name) @namespace





;; Variables and Symbols
(typed_binding (atom (qid) @variable))
(untyped_binding) @variable
(typed_binding (expr) @type)
; todo

;; Functions
(id) @function
(bid) @function
(function_name (atom (qid) @function))
(field_name) @function

;; Functions
[(data_name) (record_name)] @constructor

(SetN) @type.builtin

;; Keywords
[
  "where"
  "data"
  "rewrite"
  "postulate"
  "public"
  "private"
  "tactic"
  "Prop"
  "quote"
  "renaming"
  "open"
  "in"
  "hiding"
  "constructor"
  "abstract"
  "let"
  "field"
  "mutual"
  "module"
  "infix"
  "infixl"
  "infixr"
  "record"
] @keyword

; postulate??
; Prop??
;
; = | -> : ? \ .. ...
; (_LAMBDA) (_FORALL) (_ARROW)
; "coinductive"
; "do"
; "eta-equality"
; "field"
; "forall"
; "import"
; "inductive"
; "instance"
; "interleaved"
; "macro"
; "no-eta-equality"
; "overlap"
; "pattern"
; "primitive"
; "quoteTerm"
; "rewrite"
; "Set"
; "syntax"
; "unquote"
; "unquoteDecl"
; "unquoteDef"
; "using"
; "variable"
; "with"

; function_name
; postulate
; function
; type_signature
; field_name
; pattern
; id
; untyped_binding
; bid
; typed_binding
; primitive
; private
; record_name
; record_signature
; record
; record_assignments
; field_assignment
; module_assignment
; renaming
; import_directive
; lambda
; let
; instance
; generalize
; signature
; record
; fields
; syntax
; hole_name
; data_signature
; data_name
; data

;; Brackets
; [
;   "("
;   ")"
;   "["
;   "]"
;   "{"
;   "}"
;   "{-#"
;   "#-}"
;   "{-"
;   "-}"
;   "{!"
;   "!}"
; ] @punctuation.bracket
