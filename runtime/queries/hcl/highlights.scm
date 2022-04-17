(ERROR) @error

; { key: val }

(object_elem val: (expression
  (variable_expr
      (identifier) @type.builtin (#match? @type.builtin "^(bool|string|number|object|tuple|list|map|set|any)$"))))

(get_attr (identifier) @variable.builtin (#match? @variable.builtin  "^(root|cwd|module)$"))
(variable_expr (identifier) @variable.builtin (#match? @variable.builtin "^(var|local|path)$"))
((identifier) @type.builtin (#match? @type.builtin "^(bool|string|number|object|tuple|list|map|set|any)$"))
((identifier) @keyword (#match? @keyword "^(module|root|cwd|resource|variable|data|locals|terraform|provider|output)$"))

; highlight identifier keys as though they were block attributes
(object_elem key: (expression (variable_expr (identifier) @variable.other.member)))

(attribute (identifier) @variable.other.member)
(function_call (identifier) @function.method)
(block (identifier) @type.builtin)

(identifier) @variable
(comment) @comment
(null_lit) @constant.builtin
(numeric_lit) @constant.number
(bool_lit) @constant.builtin.boolean

[
  (template_interpolation_start) ; ${
  (template_interpolation_end) ; }
  (template_directive_start) ; %{
  (template_directive_end) ; }
  (strip_marker) ; ~
] @punctuation.special

[
  (heredoc_identifier) ; <<END
  (heredoc_start) ; END
] @punctuation.delimiter

[
  (quoted_template_start) ; "
  (quoted_template_end); "
  (template_literal) ; non-interpolation/directive content
] @string

[ 
  "if"
  "else"
  "endif"
] @keyword.control.conditional

[
  "for"
  "endfor"
  "in"
] @keyword.control.repeat

[
  ":"
  "="
] @none

[
  (ellipsis)
  "\?"
  "=>"
] @punctuation.special

[
  "."
  ".*"
  ","
  "[*]"
] @punctuation.delimiter

[
  "{"
  "}"
  "["
  "]"
  "("
  ")"
] @punctuation.bracket

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
