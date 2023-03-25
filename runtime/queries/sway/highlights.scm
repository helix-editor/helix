; -------
; Tree-Sitter doesn't allow overrides in regards to captures,
; though it is possible to affect the child node of a captured
; node. Thus, the approach here is to flip the order so that
; overrides are unnecessary.
; -------

; -------
; Types
; -------

; ---
; Primitives
; ---

(escape_sequence) @constant.character.escape
(primitive_type) @type.builtin
(boolean_literal) @constant.builtin.boolean
(integer_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float
(char_literal) @constant.character
[
  (string_literal)
  (raw_string_literal)
] @string
[
  (line_comment)
  (block_comment)
] @comment

; ---
; Extraneous
; ---

(self) @variable.builtin
(enum_variant (identifier) @type.enum.variant)

(field_initializer
  (field_identifier) @variable.other.member)
(shorthand_field_initializer
  (identifier) @variable.other.member)
(shorthand_field_identifier) @variable.other.member

(loop_label
  "'" @label
  (identifier) @label)

; ---
; Punctuation
; ---

[
  "::"
  "."
  ";"
  ","
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "#"
] @punctuation.bracket
(type_arguments
  [
    "<"
    ">"
  ] @punctuation.bracket)
(type_parameters
  [
    "<"
    ">"
  ] @punctuation.bracket)
(closure_parameters
  "|" @punctuation.bracket)

; ---
; Variables
; ---

(let_declaration
  pattern: [
    ((identifier) @variable)
    ((tuple_pattern
      (identifier) @variable))
  ])
  
; It needs to be anonymous to not conflict with `call_expression` further below. 
(_
 value: (field_expression
  value: (identifier)? @variable
  field: (field_identifier) @variable.other.member))

(parameter
	pattern: (identifier) @variable.parameter)
(closure_parameters
	(identifier) @variable.parameter)

; -------
; Keywords
; -------

(for_expression
  "for" @keyword.control.repeat)
((identifier) @keyword.control
  (#match? @keyword.control "^yield$"))

"in" @keyword.control

[
  "match"
  "if"
  "else"
] @keyword.control.conditional

[
  "while"
] @keyword.control.repeat

[
  "break"
  "continue"
  "return"
] @keyword.control.return

[
  "contract"
  "script"
  "predicate"
] @keyword.other

"use" @keyword.control.import
(dep_item "dep" @keyword.control.import !body)
(use_as_clause "as" @keyword.control.import)

(type_cast_expression "as" @keyword.operator)

[
  "as"
  "pub"
  "dep"

  "abi"
  "impl"
  "where"
  "trait"
  "for"
] @keyword

[
  "struct"
  "enum"
  "storage"
  "configurable"
] @keyword.storage.type

"let" @keyword.storage
"fn" @keyword.function
"abi" @keyword.function

(mutable_specifier) @keyword.storage.modifier.mut

(reference_type "&" @keyword.storage.modifier.ref)
(self_parameter "&" @keyword.storage.modifier.ref)

[
  "const"
  "ref"
  "deref"
  "move"
] @keyword.storage.modifier

; TODO: variable.mut to highlight mutable identifiers via locals.scm

; -------
; Guess Other Types
; -------

((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]*$"))

; ---
; PascalCase identifiers in call_expressions (e.g. `Ok()`)
; are assumed to be enum constructors.
; ---

(call_expression
  function: [
    ((identifier) @type.enum.variant
      (#match? @type.enum.variant "^[A-Z]"))
    (scoped_identifier
      name: ((identifier) @type.enum.variant
        (#match? @type.enum.variant "^[A-Z]")))
  ])

; ---
; Assume that types in match arms are enums and not
; tuple structs. Same for `if let` expressions.
; ---

(match_pattern
    (scoped_identifier
      name: (identifier) @constructor))
(tuple_struct_pattern
    type: [
      ((identifier) @constructor)
      (scoped_identifier  
        name: (identifier) @constructor)
      ])
(struct_pattern
  type: [
    ((type_identifier) @constructor)
    (scoped_type_identifier
      name: (type_identifier) @constructor)
    ])

; ---
; Other PascalCase identifiers are assumed to be structs.
; ---

((identifier) @type
  (#match? @type "^[A-Z]"))

; -------
; Functions
; -------

(call_expression
  function: [
    ((identifier) @function)
    (scoped_identifier
      name: (identifier) @function)
    (field_expression
      field: (field_identifier) @function)
  ])
(generic_function
  function: [
    ((identifier) @function)
    (scoped_identifier
      name: (identifier) @function)
    (field_expression
      field: (field_identifier) @function.method)
  ])

(function_item
  name: (identifier) @function)

(function_signature_item
  name: (identifier) @function)

; -------
; Operators
; -------

[
  "*"
  "'"
  "->"
  "=>"
  "<="
  "="
  "=="
  "!"
  "!="
  "%"
  "%="
  "&"
  "&="
  "&&"
  "|"
  "|="
  "||"
  "^"
  "^="
  "*"
  "*="
  "-"
  "-="
  "+"
  "+="
  "/"
  "/="
  ">"
  "<"
  ">="
  ">>"
  "<<"
  ">>="
  "<<="
  "@"
  ".."
  "..="
  "'"
] @operator

; -------
; Paths
; -------

(use_declaration
  argument: (identifier) @namespace)
(use_wildcard
  (identifier) @namespace)
(dep_item
  name: (identifier) @namespace)
(scoped_use_list
  path: (identifier)? @namespace)
(use_list
  (identifier) @namespace)
(use_as_clause
  path: (identifier)? @namespace
  alias: (identifier) @namespace)

; ---
; Remaining Paths
; ---

(scoped_identifier
  path: (identifier)? @namespace
  name: (identifier) @namespace)
(scoped_type_identifier
  path: (identifier) @namespace)

; -------
; Remaining Identifiers
; -------

"?" @special

(type_identifier) @type
(identifier) @variable
(field_identifier) @variable.other.member
