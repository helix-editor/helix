;; Constants, Comments, and Literals

(comment) @comment.line
(block_comment) @comment.block
[
  (documentation_comment)
  (block_documentation_comment)
] @comment.block.documentation

(nil_literal) @constant.builtin
((identifier) @constant.builtin.boolean
  (#any-of? @constant.builtin.boolean "true" "false" "on" "off"))

(char_literal) @constant.character
(escape_sequence) @constant.character.escape
(custom_numeric_literal) @constant.numeric
(integer_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float
; literals
; todo: literal?

[
  (long_string_literal)
  (raw_string_literal)
  (generalized_string)
  (interpreted_string_literal)
] @string
; (generalized_string (string_content) @none) ; todo: attempt to un-match string_content
; [] @string.regexp

[
  "."
  ","
  ";"
  ":"
] @punctuation.delimiter
[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "{."
  ".}"
] @punctuation.bracket
; todo: interpolated_str_lit?? & { }?

[
  "and"
  "or"
  "xor"
  "not"
  "div"
  "mod"
  "shl"
  "shr"
  "from"
  "as"
  "of"
  "in"
  "notin"
  "is"
  "isnot"
] @keyword.operator

[(operator) "="] @operator
(infix_expression operator: _ @operator)
(prefix_expression operator: _ @operator)

(pragma_list
  (identifier)? @attribute
  (colon_expression
    (identifier) @attribute)?)

;; Imports and Exports

[
  "import"
  "export"
  "include"
  "from"
] @keyword.control.import

(import_statement
  [
    (identifier) @namespace
    (expression_list (identifier) @namespace)
    (except_clause
      "except" @keyword.control.import
      (expression_list (identifier) @namespace))])
(import_from_statement
  (identifier) @namespace
  (expression_list (identifier) @namespace))
(include_statement (expression_list (identifier) @namespace))
(export_statement (expression_list (identifier) @namespace))

;; Control Flow

[
  "if"
  "when"
  "case"
  "elif"
  "else"
] @keyword.control.conditional
(of_branch "of" @keyword.control.conditional)
; conditional statements
; todo: do block

"block" @keyword.control
(block label: (_) @label)

[
  "for"
  "while"
  "continue"
  "break"
] @keyword.control.repeat
(for "in" @keyword.control.repeat)

[
  "return"
  "yield"
] @keyword.control.return
; return statements

[
  "try"
  "except"
  "finally"
  "raise"
] @keyword.control.exception
; exception handling statements

[
  "asm"
  "bind"
  "mixin"
  "defer"
  "static"
] @keyword
; miscellaneous keywords

;; Types and Type Declarations

[
  "let"
  "var"
  "const"
  "type"
  "object"
  "tuple"
  "enum"
  "concept"
] @keyword.storage.type

(var_type "var" @keyword.storage.modifier)
(out_type "out" @keyword.storage.modifier)
(distinct_type "distinct" @keyword.storage.modifier)
(ref_type "ref" @keyword.storage.modifier)
(pointer_type "ptr" @keyword.storage.modifier)

(var_parameter "var" @keyword.storage.modifier)
(type_parameter "type" @keyword.storage.modifier)
(static_parameter "static" @keyword.storage.modifier)
(ref_parameter "ref" @keyword.storage.modifier)
(pointer_parameter "ptr" @keyword.storage.modifier)
; (var_parameter (identifier) @variable.parameter)
; (type_parameter (identifier) @variable.parameter)
; (static_parameter (identifier) @variable.parameter)
; (ref_parameter (identifier) @variable.parameter)
; (pointer_parameter (identifier) @variable.parameter)
; todo: when are these used??

(type_section
  (type_declaration
    (type_symbol_declaration
      name: (_) @type)))
; types in type declarations

(enum_field_declaration
  (symbol_declaration
    name: (_) @type.enum.variant))
; types as enum variants

(variant_declaration
  alternative: (of_branch
    values: (expression_list (_) @type.enum.variant)))
; types as object variants

(case
  (of_branch
    values: (expression_list (_) @constant)))
; case values are guaranteed to be constant

(type_expression
  [
    (identifier) @type
    (bracket_expression
      [
        (identifier) @type
        (argument_list (identifier) @type)])
    (tuple_construction
      [
        (identifier) @type
        (bracket_expression
          [
            (identifier) @type
            (argument_list (identifier) @type)])])])
; types in type expressions

(call
  function: (bracket_expression
    right: (argument_list (identifier) @type)))
; types as generic parameters

; (dot_generic_call
;   generic_arguments: (_) @type)
; ???

(infix_expression
  operator:
    [
      "is"
      "isnot"
    ]
  right: (_) @type)
; types in "is" comparisions

(except_branch
  values: (expression_list
    [
      (identifier) @type
      (infix_expression
        left: (identifier) @type
        operator: "as"
        right: (_) @variable)]))
; types in exception branches

;; Functions

[
  "proc"
  "func"
  "method"
  "converter"
  "iterator"
  "template"
  "macro"
] @keyword.function

(exported_symbol "*" @attribute)
(_ "=" @punctuation.delimiter [body: (_) value: (_)])

(proc_declaration name: (_) @function)
(func_declaration name: (_) @function)
(iterator_declaration name: (_) @function)
(converter_declaration name: (_) @function)
(method_declaration name: (_) @function.method)
(template_declaration name: (_) @function.macro)
(macro_declaration name: (_) @function.macro)
(symbol_declaration name: (_) @variable)

(call
  function: [
    (identifier) @function.call
    (dot_expression
      right: (identifier) @function.call)
    (bracket_expression
      left: [
        (identifier) @function.call
        (dot_expression
          right: (identifier) @function.call)])])
(generalized_string
  function: [
    (identifier) @function.call
    (dot_expression
      right: (identifier) @function.call)
    (bracket_expression
      left: [
        (identifier) @function.call
        (dot_expression
          right: (identifier) @function.call)])])
(dot_generic_call function: (_) @function.call)

;; Variables

(parameter_declaration
  (symbol_declaration_list
    (symbol_declaration
      name: (_) @variable.parameter)))
(argument_list
  (equal_expression
    left: (_) @variable.parameter))
(concept_declaration
  parameters: (parameter_list (identifier) @variable.parameter))

(field_declaration
  (symbol_declaration_list
    (symbol_declaration
      name: (_) @variable.other.member)))
(call
  (argument_list
    (colon_expression
      left: (_) @variable.other.member)))
(tuple_construction
  (colon_expression
    left: (_) @variable.other.member))
(variant_declaration
  (variant_discriminator_declaration
    (symbol_declaration_list
      (symbol_declaration
        name: (_) @variable.other.member))))

;; Miscellaneous Matches

[
  "cast"
  "discard"
  "do"
] @keyword
; also: addr end interface using

(blank_identifier) @variable.builtin
((identifier) @variable.builtin
  (#eq? @variable.builtin "result"))

(dot_expression
  left: (identifier) @variable
  right: (identifier) @variable.other.member)

(identifier) @variable
