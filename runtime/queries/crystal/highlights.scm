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

(nil) @constant.builtin
(bool) @constant.builtin.boolean

(integer) @constant.numeric.integer
(float) @constant.numeric.float

[
  (string)
  (char)
  (commandLiteral)
] @string

(symbol) @string.special.symbol

(regex) @string.special.regex

; variables

(local_variable) @variable

[
  (instance_variable)
  (class_variable)
] @variable.other.member

(constant) @constant

; type defintitions

(type_identifier) @constructor

; method definition/call
(identifier) @function.method

; types
(generic_type) @type
(union_type) @type
(type_identifier) @type

