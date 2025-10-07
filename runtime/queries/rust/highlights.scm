; -------
; Basic identifiers
; -------

; We do not style ? as an operator on purpose as it allows styling ? differently, as many highlighters do. @operator.special might have been a better scope, but @special is already documented so the change would break themes (including the intent of the default theme)
"?" @special

(type_identifier) @type
(identifier) @variable
(field_identifier) @variable.other.member

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

; -------
; Types
; -------

(type_parameter
  name: (type_identifier) @type.parameter)
((type_arguments (type_identifier) @constant)
 (#match? @constant "^[A-Z_]+$"))
(type_arguments (type_identifier) @type)
; `_` in `(_, _)`
(tuple_struct_pattern "_" @comment.unused)
; `_` in `Vec<_>`
((type_arguments (type_identifier) @comment.unused)
 (#eq? @comment.unused "_"))
; `_` in `Rc<[_]>`
((array_type (type_identifier) @comment.unused)
 (#eq? @comment.unused "_"))

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

; -------
; Comments
; -------

(shebang) @comment
(line_comment) @comment.line
(block_comment) @comment.block

; Doc Comments
(line_comment
  (outer_doc_comment_marker "/" @comment.line.documentation)
  (doc_comment)) @comment.line.documentation
(line_comment
  (inner_doc_comment_marker "!" @comment.line.documentation)
  (doc_comment)) @comment.line.documentation

(block_comment
  (outer_doc_comment_marker) @comment.block.documentation
  (doc_comment) "*/" @comment.block.documentation) @comment.block.documentation
(block_comment
  (inner_doc_comment_marker) @comment.block.documentation
  (doc_comment) "*/" @comment.block.documentation) @comment.block.documentation

; ---
; Extraneous
; ---

(self) @variable.builtin

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
(for_lifetimes ["<" ">"] @punctuation.bracket)
(closure_parameters
  "|" @punctuation.bracket)
(bracketed_type ["<" ">"] @punctuation.bracket)

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
  "yield"
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

(for_expression
  "for" @keyword.control.repeat)
(gen_block "gen" @keyword.control)

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
  "raw"
  "ref"
  "move"
  "dyn"
] @keyword.storage.modifier

; TODO: variable.mut to highlight mutable identifiers via locals.scm

; ---
; Remaining Paths
; ---

(scoped_identifier
  path: (identifier)? @namespace
  name: (identifier) @namespace)
(scoped_type_identifier
  path: (identifier) @namespace)

; -------
; Functions
; -------

; highlight `baz` in `any_function(foo::bar::baz)` as function
; This generically works for an unlimited number of path segments:
;
; - `f(foo::bar)`
; - `f(foo::bar::baz)`
; - `f(foo::bar::baz::quux)`
;
; We know that in the above examples, the last component of each path is a function
; as the only other valid thing (following Rust naming conventions) would be a module at
; that position, however you cannot pass modules as arguments
(call_expression
  function: _
  arguments: (arguments
    (scoped_identifier
      path: _
      name: (identifier) @function)))

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
; Guess Other Types
; -------
; Other PascalCase identifiers are assumed to be structs.

((identifier) @type
  (#match? @type "^[A-Z]"))

(never_type "!" @type)

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

(enum_variant (identifier) @type.enum.variant)


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

; ---
; Macros
; ---

(attribute
  (identifier) @function.macro)
(inner_attribute_item "!" @punctuation)
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

(attribute
  (identifier) @special
  arguments: (token_tree (identifier) @type)
  (#eq? @special "derive")
)

(token_repetition_pattern) @punctuation.delimiter
(token_repetition_pattern [")" "(" "$"] @punctuation.special)
(token_repetition_pattern "?" @operator)

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
