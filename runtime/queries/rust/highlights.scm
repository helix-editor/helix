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

(lifetime
  "'" @label
  (identifier) @label)
(loop_label
  (identifier) @type)

; ---
; Punctuation
; ---

[
  "::"
  "."
  ";"
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
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

(arguments
  (identifier) @variable.parameter)
(parameter
	pattern: (identifier) @variable.parameter)
(closure_parameters
	(identifier) @variable.parameter)



; -------
; Keywords
; -------

(for_expression
  "for" @keyword.control)
((identifier) @keyword.control
  (#match? @keyword.control "^yield$"))
[
  "while"
  "loop"
  "in"
  "break"
  "continue"

  "match"
  "if"
  "else"
  "return"

  "await"
] @keyword.control

"use" @keyword.control.import
(mod_item "mod" @keyword.control.import !body)
(use_as_clause "as" @keyword.control.import)

(type_cast_expression "as" @keyword.operator)

[
  (crate)
  (super)
  "as"
  "pub"
  "mod"
  "extern"

  "fn"
  "struct"
  "enum"
  "impl"
  "where"
  "trait"
  "for"

  "type"
  "union"
  "unsafe"
  "default"
  "macro_rules!"

  "let"
  "ref"
  "move"

  "dyn"
  "static"
  "const"
  "async"
] @keyword

(mutable_specifier) @keyword.mut

; TODO: variable.mut to highlight mutable identifiers via locals.scm

; -------
; Guess Other Types
; -------

((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]+$"))

; ---
; PascalCase identifiers in call_expressions (e.g. `Ok()`)
; are assumed to be enum constructors.
; ---

(call_expression
  function: [
    ((identifier) @type.variant
      (#match? @type.variant "^[A-Z]"))
    (scoped_identifier
      name: ((identifier) @type.variant
        (#match? @type.variant "^[A-Z]")))
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

; ---
; Macros
; ---
(meta_item
  (identifier) @function.macro)

(inner_attribute_item) @attribute

(macro_definition
  name: (identifier) @function.macro)
(macro_invocation
  macro: [
    ((identifier) @function.macro)
    (scoped_identifier
      name: (identifier) @function.macro)
  ]
  "!" @function.macro)

(metavariable) @variable.parameter
(fragment_specifier) @type



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
(extern_crate_declaration
  name: (identifier) @namespace)
(mod_item
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
