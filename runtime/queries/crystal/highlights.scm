[
  "class"
  "struct"
  "module"

  "def"
  "alias"
  "do"
  "end"

  "require"
  "include"
  "extend"
] @keyword

[
  "[" "]"
  "(" ")"
  "{" "}"
] @punctuation.bracket

(operator) @operator

(comment) @comment

; literals

[
  (nil)
  (bool)
] @constant.builtin

[
  (integer)
  (float)
] @number

[
  (string)
  (char)
  (commandLiteral)
] @string

(symbol) @string.special.symbol

(regex) @string.special.regex

; variables

[
  (local_variable)
] @variable

[
  (instance_variable)
  (class_variable)
] @property

(constant) @constant

; type defintitions

(type_identifier) @constructor

; method definition/call
(identifier) @function.method

; types
(generic_type) @type
(union_type) @type
(type_identifier) @type


(local_variable) @variable.local
