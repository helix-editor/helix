;; ----------------------------------------------------------------------------
;; Literals and comments

(integer) @constant.numeric.integer
(exp_negation) @constant.numeric.integer
(exp_literal (float)) @constant.numeric.float
(char) @constant.character
(string) @string

(con_unit) @constant.builtin ; unit, as in ()

(comment) @comment


;; ----------------------------------------------------------------------------
;; Punctuation

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

[
  (comma)
  ";"
] @punctuation.delimiter


;; ----------------------------------------------------------------------------
;; Keywords, operators, includes

[
  "forall"
  "∀"
] @keyword.control.repeat

(pragma) @constant.macro

[
  "if"
  "then"
  "else"
  "case"
  "of"
] @keyword.control.conditional

[
  "import"
  "qualified"
  "module"
] @keyword.control.import

[
  (operator)
  (constructor_operator)
  (type_operator)
  (tycon_arrow)
  (qualified_module)  ; grabs the `.` (dot), ex: import System.IO
  (all_names)
  (wildcard)
  "="
  "|"
  "::"
  "=>"
  "->"
  "<-"
  "\\"
  "`"
  "@"
] @operator

(module) @namespace

[
  (where)
  "let"
  "in"
  "class"
  "instance"
  "data"
  "newtype"
  "family"
  "type"
  "as"
  "hiding"
  "deriving"
  "via"
  "stock"
  "anyclass"
  "do"
  "mdo"
  "rec"
  "infix"
  "infixl"
  "infixr"
] @keyword


;; ----------------------------------------------------------------------------
;; Functions and variables

(signature name: (variable) @type)
(function
  name: (variable) @function
  patterns: (patterns))
((signature (fun)) . (function (variable) @function))
((signature (context (fun))) . (function (variable) @function))
((signature (forall (context (fun)))) . (function (variable) @function))

(exp_infix (variable) @operator)  ; consider infix functions as operators

(exp_infix (exp_name) @function)
(exp_apply . (exp_name (variable) @function))
(exp_apply . (exp_name (qualified_variable (variable) @function)))

(variable) @variable
(pat_wildcard) @variable

;; ----------------------------------------------------------------------------
;; Types

(type) @type
(type_variable) @type.parameter

(constructor) @constructor

; True or False
((constructor) @_bool (#match? @_bool "(True|False)")) @constant.builtin.boolean

;; ----------------------------------------------------------------------------
;; Quasi-quotes

(quoter) @function
; Highlighting of quasiquote_body is handled by injections.scm
