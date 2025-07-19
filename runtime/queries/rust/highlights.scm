; -------
; Tree-Sitter doesn't allow overrides in regards to captures,
; though it is possible to affect the child node of a captured
; node. Thus, the approach here is to flip the order so that
; overrides are unnecessary.
; -------

; -------
; Types
; -------

(type_parameters
  (type_identifier) @type.parameter)
(constrained_type_parameter
  left: (type_identifier) @type.parameter)
(optional_type_parameter
  name: (type_identifier) @type.parameter)

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
(label
  "'" @label
  (identifier) @label)

; ---
; Prelude
; ---

((identifier) @type.enum.variant.builtin
 (#any-of? @type.enum.variant.builtin "Some" "None" "Ok" "Err"))


(call_expression
  (identifier) @function.builtin
  (#any-of? @function.builtin
    "drop"
    "size_of"
    "size_of_val"
    "align_of"
    "align_of_val"))

((type_identifier) @type.builtin
 (#any-of?
    @type.builtin
    "Send"
    "Sized"
    "Sync"
    "Unpin"
    "Drop"
    "Fn"
    "FnMut"
    "FnOnce"
    "AsMut"
    "AsRef"
    "From"
    "Into"
    "DoubleEndedIterator"
    "ExactSizeIterator"
    "Extend"
    "IntoIterator"
    "Iterator"
    "Option"
    "Result"
    "Clone"
    "Copy"
    "Debug"
    "Default"
    "Eq"
    "Hash"
    "Ord"
    "PartialEq"
    "PartialOrd"
    "ToOwned"
    "Box"
    "String"
    "ToString"
    "Vec"
    "FromIterator"
    "TryFrom"
    "TryInto"))

; ---
; Punctuation
; ---

[
  "::"
  "."
  ";"
  ","
  ":"
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
  "try"
] @keyword.control.conditional

[
  "while"
  "loop"
] @keyword.control.repeat

[
  "break"
  "continue"
  "return"
  "await"
] @keyword.control.return

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

  "impl"
  "where"
  "trait"
  "for"

  "default"
  "async"
] @keyword

[
  "struct"
  "enum"
  "union"
  "type"
] @keyword.storage.type

"let" @keyword.storage
"fn" @keyword.function
"unsafe" @keyword.special
"macro_rules!" @function.macro

(mutable_specifier) @keyword.storage.modifier.mut

(reference_type "&" @keyword.storage.modifier.ref)
(self_parameter "&" @keyword.storage.modifier.ref)

[
  "static"
  "const"
  "ref"
  "move"
  "dyn"
] @keyword.storage.modifier

; TODO: variable.mut to highlight mutable identifiers via locals.scm

; -------
; Constructors
; -------
; TODO: this is largely guesswork, remove it once we get actual info from locals.scm or r-a

(struct_expression
  name: (type_identifier) @constructor)

(tuple_struct_pattern
  type: [
    (identifier) @constructor
    (scoped_identifier
      name: (identifier) @constructor)
  ])
(struct_pattern
  type: [
    ((type_identifier) @constructor)
    (scoped_type_identifier
      name: (type_identifier) @constructor)
  ])
(match_pattern
  ((identifier) @constructor) (#match? @constructor "^[A-Z]"))
(or_pattern
  ((identifier) @constructor)
  ((identifier) @constructor)
  (#match? @constructor "^[A-Z]"))

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
    ((identifier) @constructor
      (#match? @constructor "^[A-Z]"))
    (scoped_identifier
      name: ((identifier) @constructor
        (#match? @constructor "^[A-Z]")))
  ])

; ---
; PascalCase identifiers under a path which is also PascalCase
; are assumed to be constructors if they have methods or fields.
; ---

(field_expression
  value: (scoped_identifier
    path: [
      (identifier) @type
      (scoped_identifier
        name: (identifier) @type)
    ]
    name: (identifier) @constructor
      (#match? @type "^[A-Z]")
      (#match? @constructor "^[A-Z]")))

; ---
; Other PascalCase identifiers are assumed to be structs.
; ---

((identifier) @type
  (#match? @type "^[A-Z]"))

(never_type "!" @type)

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

; ---
; Macros
; ---

(attribute
  (identifier) @special
  arguments: (token_tree (identifier) @type)
  (#eq? @special "derive")
)

(attribute
  (identifier) @function.macro)
(attribute
  [
    (identifier) @function.macro
    (scoped_identifier
      name: (identifier) @function.macro)
  ]
  (token_tree (identifier) @function.macro)?)

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
(extern_crate_declaration
  name: (identifier) @namespace
  alias: (identifier)? @namespace)
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

; We do not style ? as an operator on purpose as it allows styling ? differently, as many highlighters do. @operator.special might have been a better scope, but @special is already documented so the change would break themes (including the intent of the default theme)
"?" @special

(type_identifier) @type
(identifier) @variable
(field_identifier) @variable.other.member
