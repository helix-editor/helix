; Bottom-up approach, granular to broad.

; Guaranteed Global

"$" @punctuation.delimiter
"::" @punctuation.delimiter
"." @punctuation.delimiter
";" @punctuation.delimiter

( "#" "!" ) @punctuation.delimiter
"#" @punctuation.delimiter
"?" @punctuation.delimiter

(lifetime
  "'" @label
  (identifier) @label)
(loop_label
  (identifier) @type)

(line_comment) @comment
(block_comment) @comment

(self) @variable.builtin
(primitive_type) @type.builtin

[
(char_literal)
(string_literal)
(raw_string_literal)
] @string

(boolean_literal) @constant.builtin
(integer_literal) @number
(float_literal) @number

(escape_sequence) @escape

(attribute_item) @attribute
(inner_attribute_item) @attribute

"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket

(type_arguments
  "<" @punctuation.bracket
  ">" @punctuation.bracket)
(type_parameters
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

(parameter
	pattern: (identifier) @variable.parameter)
(closure_parameters
	(identifier) @variable.parameter)


; Other Global


(impl_item
  "for" @keyword)

"loop" @special
"for" @special
"in" @special
"break" @special
"continue" @special
"while" @special

"match" @special
"if" @special
"else" @special
"await" @special
"return" @special

(crate) @keyword
"extern" @keyword
"async" @keyword
"dyn" @keyword
"const" @keyword
"pub" @keyword
"static" @keyword

"mod" @keyword
"fn" @keyword
"enum" @keyword
"impl" @keyword
"where" @keyword
"struct" @keyword

"default" @keyword

"let" @keyword
"ref" @keyword
"move" @keyword

"macro_rules!" @keyword
"trait" @keyword
"type" @keyword
"union" @keyword
"unsafe" @keyword
"use" @keyword
(mutable_specifier) @keyword.mut
(super) @keyword
"as" @keyword


; Types

; Match identifiers that are enum variants so they don't
; get incorrectly highlighted by another query.
(enum_variant (identifier) @variant)

; Match statement for enums and tuple structs, since it's
; difficult to tell them apart, but enums are more common.
(match_pattern
  [
    (scoped_identifier
      name: (identifier) @variant)
    (tuple_struct_pattern
      (scoped_identifier
        name: (identifier) @variant))
  ])

(type_identifier) @type
(field_initializer
  (field_identifier) @property)
(shorthand_field_initializer) @variable

; Assume SCREAM_CASE identifiers are constants
((identifier) @constant
 (#match? @constant "^[A-Z](_|[A-Z])+$"))

; PascalCase identifiers in call_expressions are assumed
; to be enum constructors, everything else is assumed to be
; a type.
(call_expression
  ((identifier) @constructor
    (#match? @constructor "^[A-Z]")))
((identifier) @type
  (#match? @type "^[A-Z]"))

(meta_item
  (identifier) @attribute)


; Functions


; Macros
(macro_definition
  name: (identifier) @function.macro)
(macro_invocation
  macro: [
    ((identifier) @function.macro)
    (scoped_identifier
      name: (identifier) @function.macro)
  ])

(metavariable) @variable.parameter
(fragment_specifier) @variable.parameter

; Others

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


; Paths


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

; Global Paths

(scoped_identifier
  path: (identifier)? @namespace
  name: (identifier) @namespace)
(scoped_type_identifier
  path: (identifier)? @namespace
  name: (type_identifier) @type)


; Remaining Globals


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

; Not sure why I have to rewrite it here,
; but this duplicate is needed.
(type_identifier) @type
(identifier) @variable
(field_identifier) @variable
