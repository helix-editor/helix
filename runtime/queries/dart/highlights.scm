(dotted_identifier_list) @string

; Methods
; --------------------
(super) @function.builtin

(function_expression_body (identifier) @function.method)
((identifier)(selector (argument_part)) @function.method)

; Annotations
; --------------------
(annotation
  name: (identifier) @attribute)
(marker_annotation
  name: (identifier) @attribute)

; Types
; --------------------
(class_definition
  name: (identifier) @type)
  
(constructor_signature
  name: (identifier) @function.method)

(function_signature
  name: (identifier) @function.method)

(getter_signature
  (identifier) @function.builtin)

(setter_signature
  name: (identifier) @function.builtin)

(enum_declaration
  name: (identifier) @type)

(enum_constant
  name: (identifier) @type.builtin)

(void_type) @type.builtin

((scoped_identifier
  scope: (identifier) @type)
 (#match? @type "^[a-zA-Z]"))
 
((scoped_identifier
  scope: (identifier) @type
  name: (identifier) @type)
 (#match? @type "^[a-zA-Z]"))

; the DisabledDrawerButtons in : const DisabledDrawerButtons(history: true),
(type_identifier) @type.builtin

; Variables
; --------------------
; the "File" in var file = File();
((identifier) @namespace
 (#match? @namespace "^_?[A-Z].*[a-z]")) ; catch Classes or IClasses not CLASSES

("Function" @type.builtin)
(inferred_type) @type.builtin

; properties
(unconditional_assignable_selector
  (identifier) @variable.other.member)

(conditional_assignable_selector
  (identifier) @variable.other.member)

; assignments
; --------------------
; the "strings" in : strings = "some string"
(assignment_expression
  left: (assignable_expression) @variable)

(this) @variable.builtin

; Parameters
; --------------------
(formal_parameter
    name: (identifier) @variable)

(named_argument
  (label (identifier) @variable))

; Literals
; --------------------
[
  (hex_integer_literal)
  (decimal_integer_literal)
  (decimal_floating_point_literal)
  ;(octal_integer_literal)
  ;(hex_floating_point_literal)
] @constant.numeric.integer

(symbol_literal) @string.special.symbol
(string_literal) @string

[
  (const_builtin)
  (final_builtin)
] @variable.builtin

[
  (true)
  (false)
] @constant.builtin.boolean

(null_literal) @constant.builtin

(comment) @comment.line
(documentation_comment) @comment.block.documentation

; Tokens
; --------------------
(template_substitution
  "$" @punctuation.special
  "{" @punctuation.special
  "}" @punctuation.special
) @embedded

(template_substitution
  "$" @punctuation.special
  (identifier_dollar_escaped) @variable
) @embedded

(escape_sequence) @constant.character.escape

; Punctuation
;---------------------
[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket

[
  ";"
  "."
  ","
  ":"
] @punctuation.delimiter
  
; Operators
;---------------------
[
 "@"
 "?"
 "=>"
 ".."
 "=="
 "&&"
 "%"
 "<"
 ">"
 "="
 ">="
 "<="
 "||"
 (multiplicative_operator)
 (increment_operator)
 (is_operator)
 (prefix_operator)
 (equality_operator)
 (additive_operator)
] @operator

; Keywords
; --------------------
["import" "library" "export"] @keyword.control.import
["do" "while" "continue" "for"] @keyword.control.repeat
["return" "yield"] @keyword.control.return
["as" "in" "is"] @keyword.operator

[
  "?."
  "??"
  "if"
  "else"
  "switch"
  "default"
  "late"
] @keyword.control.conditional

[
  "try"
  "throw"
  "catch"
  "finally"
  (break_statement)
] @keyword.control.exception

; Reserved words (cannot be used as identifiers)
[
    (case_builtin)
    "abstract"
    "async"
    "async*"
    "await"
    "base"
    "class"
    "covariant"
    "deferred"
    "dynamic"
    "enum"
    "extends"
    "extension"
    "external"
    "factory"
    "Function"
    "get"
    "implements"
    "interface"
    "mixin"
    "new"
    "on"
    "operator"
    "part"
    "required"
    "sealed"
    "set"
    "show"
    "static"
    "super"
    "sync*"
    "typedef"
    "with"
] @keyword

; when used as an identifier:
((identifier) @variable.builtin
 (#match? @variable.builtin "^(abstract|as|base|covariant|deferred|dynamic|export|external|factory|Function|get|implements|import|interface|library|operator|mixin|part|sealed|set|static|typedef)$"))
