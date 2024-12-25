;; Punctuation
[ "." ";" ":"] @punctuation.delimiter
[ "(" ")" "{" "}" ] @punctuation.bracket

;; Constants
(integer) @constant.numeric.integer
; (float) @constant.numeric.float
(literal) @string

;; Pragmas and comments
(comment) @comment
(pragma) @attribute
(macro) @function.macro

;; Imports
(module_name) @namespace
(import_directive (id) @namespace)
[(module) (import) (open)] @keyword.control.import

;; Types
(typed_binding (expr) @type)
(record        (expr) @type)
(data          (expr) @type)
(signature     (expr) @type)
(function (rhs (expr) @type))
; todo: these are too general. ideally, any nested (atom)
; https://github.com/tree-sitter/tree-sitter/issues/880

;; Variables
(untyped_binding (atom) @variable)
(typed_binding   (atom) @variable)
(field_name) @variable.other.member

;; Functions
(function_name) @function
;(function (lhs
;  . (atom) @function
;    (atom) @variable.parameter))
; todo: currently fails to parse, upstream tree-sitter bug

;; Data
[(data_name) (record_name)] @constructor
((atom) @constant.builtin.boolean
  (#any-of? @constant.builtin.boolean "true" "false" "True" "False"))

"Set" @type.builtin

; postulate
; type_signature
; pattern
; id
; bid
; typed_binding
; primitive
; private
; record_signature
; record_assignments
; field_assignment
; module_assignment
; renaming
; import_directive
; lambda
; let
; instance
; generalize
; record
; fields
; syntax
; hole_name
; data_signature

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
  "in"
  "hiding"
  "constructor"
  "abstract"
  "let"
  "field"
  "mutual"
  "infix"
  "infixl"
  "infixr"
  "record"
  "overlap"
  "instance"
  "do"
] @keyword

[
  "="
] @operator

; = | -> : ? \ .. ... λ ∀ →
; (_LAMBDA) (_FORALL) (_ARROW)
; "coinductive"
; "eta-equality"
; "field"
; "inductive"
; "interleaved"
; "macro"
; "no-eta-equality"
; "pattern"
; "primitive"
; "quoteTerm"
; "rewrite"
; "syntax"
; "unquote"
; "unquoteDecl"
; "unquoteDef"
; "using"
; "variable"
; "with"

