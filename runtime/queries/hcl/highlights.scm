; { key: val }
;
; highlight identifier keys as though they were block attributes
; (object_elem (identifier) () (expression (variable_expr (identifier) @field)))

; (object_elem val: (expression
;   (variable_expr
;     (identifier) @type.builtin (#any-of? @type.builtin "bool" "string" "number" "object" "tuple" "list" "map" "set" "any"))))

(get_attr (identifier) @variable.builtin (#match? @variable.builtin  "^(root|cwd|module)$"))
((identifier) @keyword (#match? @keyword "^(module|root|cwd|resource|variable|data|locals|terraform|provider|output)$"))
((identifier) @type.builtin (#match? @type.builtin "^(bool|string|number|object|tuple|list|map|set|any)$"))
(variable_expr (identifier) @variable.builtin (#match? @variable.builtin "^(var|local|path)$"))

(attribute (identifier) @field)
(function_call (identifier) @function)
(block (identifier) @type)

[
  (true)
  (false)
]  @boolean

(null) @constant
(comment) @comment
(identifier) @variable

[
  "!"
  "\*"
  "/"
  "%"
  "\+"
  "-"
  ">"
  ">="
  "<"
  "<="
  "=="
  "!="
  "&&"
  "||"
] @operator

[
  "{"
  "}"
  "["
  "]"
  "("
  ")"
] @punctuation.bracket

[
  "."
  ","
] @punctuation.delimiter

[
  "?"
  "=>"
] @punctuation.special

[
  ":"
  "="
] @none

[
  "for"
  "in"
] @repeat

[ 
  (conditional)
  "if"
] @conditional

[
  (string_literal)
  (quoted_template)
] @string

(escape_sequence) @punctuation.special
