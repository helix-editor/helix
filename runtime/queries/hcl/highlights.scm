; highlights.scm

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
  ".*"
  ","
  "[*]"
] @punctuation.delimiter

[
  (ellipsis)
  "\?"
  "=>"
] @punctuation.special

[
  ":"
  "="
] @none

[
  "for"
  "endfor"
  "in"
] @repeat

[ 
  "if"
  "else"
  "endif"
] @conditional

[
  (quoted_template_start) ; "
  (quoted_template_end); "
  (template_literal) ; non-interpolation/directive content
] @string

[
  (heredoc_identifier) ; <<END
  (heredoc_start) ; END
] @punctuation.delimiter

[
  (template_interpolation_start) ; ${
  (template_interpolation_end) ; }
  (template_directive_start) ; %{
  (template_directive_end) ; }
  (strip_marker) ; ~
] @punctuation.special

(numeric_lit) @number
(bool_lit) @boolean
(null_lit) @constant
(comment) @comment
(identifier) @variable

(block (identifier) @type)
(function_call (identifier) @function)
(attribute (identifier) @field)

; { key: val }
;
; highlight identifier keys as though they were block attributes
(object_elem key: (expression (variable_expr (identifier) @field)))

((identifier) @keyword (#any-of? @keyword "module" "root" "cwd" "resource" "variable" "data" "locals" "terraform" "provider" "output"))
((identifier) @type.builtin (#any-of? @type.builtin "bool" "string" "number" "object" "tuple" "list" "map" "set" "any"))
(variable_expr (identifier) @variable.builtin (#any-of? @variable.builtin "var" "local" "path"))
(get_attr (identifier) @variable.builtin (#any-of? @variable.builtin  "root" "cwd" "module"))

(object_elem val: (expression
  (variable_expr
    (identifier) @type.builtin (#any-of? @type.builtin "bool" "string" "number" "object" "tuple" "list" "map" "set" "any"))))

(ERROR) @error
