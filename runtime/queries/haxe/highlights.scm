(identifier) @type
(comment) @comment

; MetaData
; --------
(metadata) @tag
(metadata name: (identifier) @type) @tag

; Declarations
; ------------
(import_statement name: (identifier) @type)
(package_statement name: (identifier) @type)

(class_declaration name: (identifier) @type)
(class_declaration (type_params (type_param (identifier) @type)))

(function_declaration name: (identifier) @function)
(function_arg name: (identifier) @variable.parameter)

; Generic/Type Params
; --------------
(type_params
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

; Literals
; --------
[(null) (keyword)] @keyword
[(type) (literal)] @type
[(builtin_type)] @type.builtin
[(integer) (float)] @number
(string) @string
(bool) @constant
(operator) @operator

; Interpolation
; -------------
(interpolation "$" @punctuation.special)
(interpolation
  "${" @punctuation.special
  "}" @punctuation.special
) @embedded


; Tokens
; ------

(":") @punctuation.special

[
  "("
  ")"
;   "["
;   "]"
  "{"
  "}"
]  @punctuation.bracket
;
[
;   ";"
;   "?."
;   "."
  ","
] @punctuation.delimiter

; (variable_declaration name: (identifier) @number)
; (variable_declaration (type) @type)
; (increment_operator) @operator
; (decrement_operator) @operator
; (decrement_unop (identifier) (decrement_operator))
; (decrement_unop (identifier) (decrement_operator))
