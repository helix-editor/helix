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

(qualified_module (module) @constructor)
(qualified_type (module) @namespace)
(qualified_variable (module) @namespace)
(import (module) @namespace)

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
  "forall"
  "∀"
  "infix"
  "infixl"
  "infixr"
] @keyword


;; ----------------------------------------------------------------------------
;; Functions and variables

(signature name: (variable) @type)
(function name: (variable) @function)

(variable) @variable
"_" @variable.builtin

(exp_infix (variable) @operator)  ; consider infix functions as operators

("@" @namespace)  ; "as" pattern operator, e.g. x@Constructor


;; ----------------------------------------------------------------------------
;; Types

(type) @type

(constructor) @constructor

; True or False
((constructor) @_bool (#match? @_bool "(True|False)")) @constant.builtin.boolean
