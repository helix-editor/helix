;; ----------------------------------------------------------------------------
;; Literals and comments

(integer) @constant.numeric.integer
(float) @constant.numeric.float
(char) @constant.character
(string) @string
(attribute_name) @attribute
(attribute_exclamation_mark) @attribute

(con_unit) @constant.builtin ; unit, as in ()

(comment) @comment

;; ----------------------------------------------------------------------------
;; Keywords, operators, includes

[
  "Id"
  "Primary"
  "Foreign"
  "deriving"
] @keyword

"=" @operator

;; ----------------------------------------------------------------------------
;; Functions and variables

(variable) @variable

;; ----------------------------------------------------------------------------
;; Types

(type) @type

(constructor) @constructor
