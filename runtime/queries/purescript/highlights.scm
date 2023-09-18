; ----------------------------------------------------------------------------
; Literals and comments

 (integer) @constant.numeric.integer
 (exp_negation) @constant.numeric.integer
 (exp_literal (float)) @constant.numeric.float
 (char) @constant.character
 (string) @string

 (con_unit) @constant.builtin ; unit, as in ()

 (comment) @comment


; ----------------------------------------------------------------------------
; Punctuation

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


; ----------------------------------------------------------------------------
; Keywords, operators, includes

 [
   "if"
   "then"
   "else"
   "case"
   "of"
 ] @keyword.control.conditional

 [
   "import"
   "module"
 ] @keyword.control.import

 [
   (operator)
   (constructor_operator)
   (type_operator)
   (tycon_arrow)
   (qualified_module)  ; grabs the `.` (dot), ex: import System.IO
   (all_names)
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
 (module) @namespace
 (qualified_type (module) @namespace)
 (qualified_variable (module) @namespace)
 (import (module) @namespace)

 [
   (where)
   "let"
   "in"
   "class"
   "instance"
   "derive"
   "foreign"
   "data"
   "newtype"
   "type"
   "as"
   "do"
   "ado"
   "forall"
   "∀"
   "infix"
   "infixl"
   "infixr"
 ] @keyword


; ----------------------------------------------------------------------------
; Functions and variables

 (signature name: (variable) @type)
 (function name: (variable) @function)

 ; true or false
((variable) @constant.builtin.boolean
 (#match? @constant.builtin.boolean "^(true|false)$"))

 (variable) @variable

 (exp_infix (variable) @operator)  ; consider infix functions as operators

 ("@" @namespace)  ; "as" pattern operator, e.g. x@Constructor


; ----------------------------------------------------------------------------
; Types

 (type) @type

 (constructor) @constructor

