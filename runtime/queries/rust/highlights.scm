; Identifier conventions


; Assume all-caps names are constants
((identifier) @constant
 (#match? @constant "^[A-Z](_|[A-Z])+$"))

; Assume other uppercase names are enum constructors
(enum_variant) @variant
((identifier) @constructor
 (#match? @constructor "^[A-Z]"))

; Assume that uppercase names in paths are types
(mod_item
 name: (identifier) @namespace)
(scoped_identifier
  path: (identifier) @namespace)
(scoped_identifier
 (scoped_identifier
  name: (identifier) @namespace))
(scoped_type_identifier
  path: (identifier) @namespace)
(scoped_type_identifier
 (scoped_identifier
  name: (identifier) @namespace))

((scoped_identifier
  path: (identifier) @type)
 (#match? @type "^[A-Z]"))
((scoped_identifier
  path: (scoped_identifier
    name: (identifier) @type))
 (#match? @type "^[A-Z]"))

; Namespaces

(crate) @namespace
(extern_crate_declaration
    (crate)
    name: (identifier) @namespace)
(scoped_use_list
  path: (identifier) @namespace)
(scoped_use_list
  path: (scoped_identifier
            (identifier) @namespace))
(use_list (scoped_identifier (identifier) @namespace . (_)))

; Function calls

(call_expression
  function: (identifier) @function)
(call_expression
  function: (field_expression
    field: (field_identifier) @function.method))
(call_expression
  function: (scoped_identifier
    "::"
    name: (identifier) @function))

(generic_function
  function: (identifier) @function)
(generic_function
  function: (scoped_identifier
    name: (identifier) @function))
(generic_function
  function: (field_expression
    field: (field_identifier) @function.method))

(macro_invocation
  macro: (identifier) @function.macro
  "!" @function.macro)
(macro_invocation
  macro: (scoped_identifier
           (identifier) @function.macro .))

; (metavariable) @variable
(metavariable) @function.macro

"$" @function.macro

; Function definitions

(function_item (identifier) @function)
(function_signature_item (identifier) @function)

; Other identifiers

(type_identifier) @type
(primitive_type) @type.builtin
(field_identifier) @property

(line_comment) @comment
(block_comment) @comment

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

"::" @punctuation.delimiter
"." @punctuation.delimiter
";" @punctuation.delimiter

(parameter (identifier) @variable.parameter)
(closure_parameters (_) @variable.parameter)

(lifetime (identifier) @label)

"async" @keyword
"break" @keyword
"const" @keyword
"continue" @keyword
(crate) @keyword
"default" @keyword
"dyn" @keyword
"else" @keyword
"enum" @keyword
"extern" @keyword
"fn" @keyword
"for" @keyword
"if" @keyword
"impl" @keyword
"in" @keyword
"let" @keyword
"let" @keyword
"loop" @keyword
"macro_rules!" @keyword
"match" @keyword
"mod" @keyword
"move" @keyword
"pub" @keyword
"ref" @keyword
"return" @keyword
"static" @keyword
"struct" @keyword
"trait" @keyword
"type" @keyword
"union" @keyword
"unsafe" @keyword
"use" @keyword
"where" @keyword
"while" @keyword
(mutable_specifier) @keyword.mut
(use_list (self) @keyword)
(scoped_use_list (self) @keyword)
(scoped_identifier (self) @keyword)
(super) @keyword
"as" @keyword

(self) @variable.builtin

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

"?" @special
