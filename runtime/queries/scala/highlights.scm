; CREDITS @stumash (stuart.mashaal@gmail.com)

;; variables


((identifier) @variable.builtin
 (#match? @variable.builtin "^this$"))

(interpolation) @none

; Assume other uppercase names constants.
; NOTE: In order to distinguish constants we highlight
; all the identifiers that are uppercased. But this solution
; is not suitable for all occurrences e.g. it will highlight
; an uppercased method as a constant if used with no params.
; Introducing highlighting for those specific cases, is probably
; best way to resolve the issue.
((identifier) @constant (#match? @constant "^[A-Z]"))

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

(full_enum_case
  name: (identifier) @type)

(simple_enum_case
  name: (identifier) @type)

;; val/var definitions/declarations

(val_definition
  pattern: (identifier) @variable)

(var_definition
  pattern: (identifier) @variable)

(val_declaration
  name: (identifier) @variable)

(var_declaration
  name: (identifier) @variable)

; function definitions/declarations

(function_declaration
    name: (identifier) @function.method)

(function_definition
      name: (identifier) @function.method)

; imports/exports

(import_declaration
  path: (identifier) @namespace)
((stable_identifier (identifier) @namespace))

((import_declaration
  path: (identifier) @type) (#match? @type "^[A-Z]"))
((stable_identifier (identifier) @type) (#match? @type "^[A-Z]"))

(export_declaration
  path: (identifier) @namespace)
((stable_identifier (identifier) @namespace))

((export_declaration
  path: (identifier) @type) (#match? @type "^[A-Z]"))
((stable_identifier (identifier) @type) (#match? @type "^[A-Z]"))

((namespace_selectors (identifier) @type) (#match? @type "^[A-Z]"))

; method invocation


(call_expression
  function: (identifier) @function)

(call_expression
  function: (operator_identifier) @function)

(call_expression
  function: (field_expression
    field: (identifier) @function.method))

(call_expression
  function: (field_expression
    field: (operator_identifier) @function.method))

((call_expression
   function: (identifier) @variable.other.member)
 (#match? @variable.other.member "^[A-Z]"))

(generic_function
  function: (identifier) @function)

(interpolated_string_expression
  interpolator: (identifier) @function)

(
  (identifier) @function.builtin
  (#match? @function.builtin "^super$")
)

; function definitions

(function_definition
  name: (identifier) @function)

(function_definition
  name: (operator_identifier) @function)

(parameter
  name: (identifier) @variable.parameter)

(binding
  name: (identifier) @variable.parameter)

; expressions


(field_expression field: (identifier) @variable.other.member)
(field_expression value: (identifier) @type
 (#match? @type "^[A-Z]"))

(infix_expression operator: (identifier) @operator)
(infix_expression operator: (operator_identifier) @operator)
(infix_type operator: (operator_identifier) @operator)
(infix_type operator: (operator_identifier) @operator)

; literals
(boolean_literal) @constant.builtin.boolean
(integer_literal) @constant.numeric.integer
(floating_point_literal) @constant.numeric.float


(symbol_literal) @string.special.symbol

[
(string)
(character_literal)
(interpolated_string_expression)
] @string

(interpolation "$" @punctuation.special)

; annotations

(annotation) @attribute

;; keywords

;; storage in TextMate scope lingo means field or type
[
  (opaque_modifier)
  (infix_modifier)
  (transparent_modifier)
  (open_modifier)
  "abstract"
  "final"
  "implicit"
  "lazy"
  "override"
  "private"
  "protected"
  "sealed"
] @keyword.storage.modifier

[
  "class"
  "enum"
  "extension"
  "given"
  "object"
  "package"
  "trait"
  "type"
  "val"
  "var"
] @keyword.storage.type

[
  "as"
  "derives"
  "end"
  "extends"
;; `forSome` existential types not implemented yet
;; `macro` not implemented yet
;; `throws`
  "using"
  "with"
] @keyword

(null_literal) @constant.builtin
(wildcard) @keyword

;; special keywords

"new" @keyword.operator

[
  "case"
  "catch"
  "else"
  "finally"
  "if"
  "match"
  "then"
  "throw"
  "try"
] @keyword.control.conditional

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  "."
  ","
] @punctuation.delimiter

[
  "do"
  "for"
  "while"
  "yield"
] @keyword.control.repeat

"def" @keyword.function

[
  "=>"
  "<-"
  "@"
] @keyword.operator

"import" @keyword.control.import

"export" @keyword.control.import

"return" @keyword.control.return

[(comment) (block_comment)] @comment

;; `case` is a conditional keyword in case_block

(case_block
  (case_clause ("case") @keyword.control.conditional))

(identifier) @variable

(operator_identifier) @operator
