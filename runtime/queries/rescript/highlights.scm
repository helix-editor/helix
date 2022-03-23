(comment) @comment

; Identifiers
;------------

; Escaped identifiers like \"+."
((value_identifier) @function.macro
 (#match? @function.macro "^\\.*$"))

[
  (type_identifier)
  (unit_type)
  "list"
] @type

[
  (variant_identifier)
  (polyvar_identifier)
] @constant

(property_identifier) @variable.other.member
(module_identifier) @namespace

(jsx_identifier) @tag
(jsx_attribute (property_identifier) @variable.parameter)

; Parameters
;----------------

(list_pattern (value_identifier) @variable.parameter)
(spread_pattern (value_identifier) @variable.parameter)

; String literals
;----------------

[
  (string)
  (template_string)
] @string

(template_substitution
  "${" @punctuation.bracket
  "}" @punctuation.bracket) @embedded

(character) @constant.character
(escape_sequence) @constant.character.escape

; Other literals
;---------------

[
  (true)
  (false)
] @constant.builtin

(number) @constant.numeric
(polyvar) @constant
(polyvar_string) @constant

; Functions
;----------

[
 (formal_parameters (value_identifier))
 (labeled_parameter (value_identifier))
] @variable.parameter

(function parameter: (value_identifier) @variable.parameter)

; Meta
;-----

[
 "@"
 "@@"
 (decorator_identifier)
] @label

(extension_identifier) @keyword
("%") @keyword

; Misc
;-----

(subscript_expression index: (string) @variable.other.member)
(polyvar_type_pattern "#" @constant)

[
  ("include")
  ("open")
] @keyword

[
  "as"
  "export"
  "external"
  "let"
  "module"
  "mutable"
  "private"
  "rec"
  "type"
  "and"
] @keyword

[
  "if"
  "else"
  "switch"
] @keyword

[
  "exception"
  "try"
  "catch"
  "raise"
] @keyword

[
  "."
  ","
  "|"
] @punctuation.delimiter

[
  "++"
  "+"
  "+."
  "-"
  "-."
  "*"
  "*."
  "/"
  "/."
  "<"
  "<="
  "=="
  "==="
  "!"
  "!="
  "!=="
  ">"
  ">="
  "&&"
  "||"
  "="
  ":="
  "->"
  "|>"
  ":>"
  (uncurry)
] @operator

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

(polyvar_type
  [
   "["
   "[>"
   "[<"
   "]"
  ] @punctuation.bracket)

[
  "~"
  "?"
  "=>"
  "..."
] @punctuation

(ternary_expression ["?" ":"] @operator)
