; CREDITS @stumash (stuart.mashaal@gmail.com)
; https://github.com/nvim-treesitter/nvim-treesitter/tree/master/queries/scala

;; variables

(identifier) @variable

((identifier) @variable.builtin
 (#lua-match? @variable.builtin "^this$"))

(interpolation) @none

; Assume other uppercase names constants.
; NOTE: In order to distinguish constants we highlight
; all the identifiers that are uppercased. But this solution
; is not suitable for all occurrences e.g. it will highlight
; an uppercased method as a constant if used with no params.
; Introducing highlighting for those specific cases, is probably
; best way to resolve the issue.
((identifier) @constant (#lua-match? @constant "^[A-Z]"))

;; types

(type_identifier) @type

(class_definition
  name: (identifier) @type)

(object_definition
  name: (identifier) @type)

(trait_definition
  name: (identifier) @type)

(type_definition
  name: (type_identifier) @type)

; method definition

(class_definition
  body: (template_body
    (function_definition
      name: (identifier) @method)))
(object_definition
  body: (template_body
    (function_definition
      name: (identifier) @method)))
(trait_definition
  body: (template_body
    (function_definition
      name: (identifier) @method)))

; imports

(import_declaration
  path: (identifier) @namespace)
((stable_identifier (identifier) @namespace))

((import_declaration
  path: (identifier) @type) (#lua-match? @type "^[A-Z]"))
((stable_identifier (identifier) @type) (#lua-match? @type "^[A-Z]"))

((import_selectors (identifier) @type) (#lua-match? @type "^[A-Z]"))

; method invocation


(call_expression
  function: (identifier) @function)

(call_expression
  function: (field_expression
    field: (identifier) @method))

((call_expression
   function: (identifier) @constructor)
 (#lua-match? @constructor "^[A-Z]"))

(generic_function
  function: (identifier) @function)

(
  (identifier) @function.builtin
  (#lua-match? @function.builtin "^super$")
)

; function definitions

(function_definition
  name: (identifier) @function)

(parameter
  name: (identifier) @parameter)

; expressions


(field_expression field: (identifier) @property)
(field_expression value: (identifier) @type
 (#lua-match? @type "^[A-Z]"))

(infix_expression operator: (identifier) @operator)
(infix_expression operator: (operator_identifier) @operator)
(infix_type operator: (operator_identifier) @operator)
(infix_type operator: (operator_identifier) @operator)

; literals

(boolean_literal) @boolean
(integer_literal) @number
(floating_point_literal) @float

[
(symbol_literal)
(string)
(character_literal)
(interpolated_string_expression)
] @string

(interpolation "$" @punctuation.special)

;; keywords

[
  "abstract"
  "case"
  "class"
  "extends"
  "final"
  "finally"
;; `forSome` existential types not implemented yet
  "implicit"
  "lazy"
;; `macro` not implemented yet
  "object"
  "override"
  "package"
  "private"
  "protected"
  "sealed"
  "trait"
  "type"
  "val"
  "var"
  "with"
] @keyword

(null_literal) @keyword
(wildcard) @keyword

;; special keywords

"new" @keyword.operator

[
  "else"
  "if"
  "match"
] @conditional

[
 "("
 ")"
 "["
 "]"
 "{"
 "}"
]  @punctuation.bracket

[
 "."
 ","
] @punctuation.delimiter

[
  "do"
  "for"
  "while"
  "yield"
] @repeat

"def" @keyword.function

[
 "=>"
 "<-"
 "@"
] @operator

"import" @include

[
  "try"
  "catch"
  "throw"
] @exception

"return" @keyword.return

(comment) @comment

;; `case` is a conditional keyword in case_block

(case_block
  (case_clause ("case") @conditional))
