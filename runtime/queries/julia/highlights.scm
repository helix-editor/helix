; ----------
; Primitives
; ----------

[
  (line_comment)
  (block_comment)
] @comment

(
  ((identifier) @constant.builtin)
  (#match? @constant.builtin "^(nothing|missing|undef)$"))

[
  (true)
  (false)
] @constant.builtin.boolean

(integer_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float

(
  ((identifier) @constant.numeric.float)
  (#match? @constant.numeric.float "^((Inf|NaN)(16|32|64)?)$"))

(character_literal) @constant.character
(escape_sequence) @constant.character.escape

(string_literal) @string

(prefixed_string_literal
  prefix: (identifier) @function.macro) @string

(quote_expression
  (identifier) @string.special.symbol) 
  
; -------------------
; Modules and Imports
; -------------------

(module_definition
  name: (identifier) @namespace)
  
(import_statement
  (identifier) @namespace)
  
(selected_import
  . (identifier) @namespace)

(scoped_identifier
  (identifier) @namespace)

; -----
; Types
; -----

(abstract_definition
  name: (identifier) @type)
  
(primitive_definition
  name: (identifier) @type)

(struct_definition
  name: (identifier) @type)

(struct_definition
  . (_)
    (identifier) @variable.other.member)

(struct_definition
  . (_)
  (typed_expression
    . (identifier) @variable.other.member))
    
(type_parameter_list
  (identifier) @type)

(constrained_type_parameter
  (identifier) @type)
  
(subtype_clause
  (identifier) @type)

(typed_expression
  (identifier) @type . )

(parameterized_identifier
  (identifier) @type)
  
(type_argument_list
  (identifier) @type)

(where_clause
  (identifier) @type)

; -------------------
; Function definition
; -------------------

(
  (function_definition
    name: [
      (identifier) @function
      (scoped_identifier
        (identifier) @namespace
        (identifier) @function)
    ])
  ; prevent constructors (PascalCase) to be highlighted as functions
  (#match? @function "^[^A-Z]"))

(
  (short_function_definition
    name: [
      (identifier) @function
      (scoped_identifier
        (identifier) @namespace
        (identifier) @function)
    ])
  ; prevent constructors (PascalCase) to be highlighted as functions
  (#match? @function "^[^A-Z]"))

(parameter_list
  (identifier) @variable.parameter)

(typed_parameter
  (identifier) @variable.parameter
  (identifier)? @type)

(optional_parameter
  . (identifier) @variable.parameter)

(slurp_parameter
  (identifier) @variable.parameter)

(function_expression
  . (identifier) @variable.parameter)

; ---------------
; Functions calls
; ---------------

(
  (call_expression
    (identifier) @function)
  ; prevent constructors (PascalCase) to be highlighted as functions
  (#match? @function "^[^A-Z]"))

(
  (broadcast_call_expression
    (identifier) @function)
  (#match? @function "^[^A-Z]"))

(
  (call_expression
    (field_expression (identifier) @function .))
  (#match? @function "^[^A-Z]"))

(
  (broadcast_call_expression
    (field_expression (identifier) @function .))
  (#match? @function "^[^A-Z]"))

; ------
; Macros
; ------

(macro_definition
  name: (identifier) @function.macro)

(macro_identifier
  "@" @function.macro
  (identifier) @function.macro)

; --------
; Keywords
; --------

(function_definition 
  ["function" "end"] @keyword.function)

(if_statement
  ["if" "end"] @keyword.control.conditional)
(elseif_clause
  ["elseif"] @keyword.control.conditional)
(else_clause
  ["else"] @keyword.control.conditional)
(ternary_expression
  ["?" ":"] @keyword.control.conditional)

(for_statement
  ["for" "end"] @keyword.control.repeat)
(while_statement
  ["while" "end"] @keyword.control.repeat)
(break_statement) @keyword.control.repeat
(continue_statement) @keyword.control.repeat
(for_binding
  "in" @keyword.control.repeat)
(for_clause
  "for" @keyword.control.repeat)

(try_statement
  ["try" "end" ] @keyword.control.exception)
(finally_clause
  "finally" @keyword.control.exception)
(catch_clause
  "catch" @keyword.control.exception)

[
  "export"
  "import"
  "using"
] @keyword.control.import

[
  "abstract"
  "baremodule"
  "begin"
  "const"
  "do"
  "end"
  "let"
  "macro"
  "module"
  "mutable"
  "primitive"
  "quote"
  "return"
  "struct"
  "type"
  "where"
] @keyword

; TODO: fix this
((identifier) @keyword (#match? @keyword "global|local"))

; ---------
; Operators
; ---------

[
  (operator)
  "::"
  "<:"
  ":"
  "=>"
  "..."
  "$"
] @operator

; ------------
; Punctuations
; ------------

[
  "."
  "," 
  ";"
] @punctuation.delimiter

[
  "["
  "]"
  "("
  ")" 
  "{" 
  "}"
] @punctuation.bracket

; ---------------------
; Remaining identifiers
; ---------------------

(const_statement
  (variable_declaration
    . (identifier) @constant))

; SCREAMING_SNAKE_CASE
(
  (identifier) @constant
  (#match? @constant "^[A-Z][A-Z0-9_]*$"))

; remaining identifiers that start with capital letters should be types (PascalCase)
(
  (identifier) @type
  (#match? @type "^[A-Z]"))

; Field expressions are either module content or struct fields.
; Module types and constants should already be captured, so this
; assumes the remaining identifiers to be struct fields.
(field_expression
  (_)
  (identifier) @variable.other.member)

(identifier) @variable
