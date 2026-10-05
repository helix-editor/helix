; Highlights for Pyfun (https://github.com/simontreanor/Pyfun).
; Generic rules first; later (more specific) patterns win on equal spans.

; ---- identifiers (defaults) ----

(identifier) @variable
(constructor_identifier) @constructor
(type_identifier) @type
(type_variable) @type.parameter
(module_identifier) @namespace
(wildcard) @comment.unused
(hole) @variable.builtin

(parameter (identifier) @variable.parameter)

; a `let` with parameters defines a function
(let_binding
  name: (identifier) @function
  parameter: (parameter))

(active_pattern_cases
  case: (constructor_identifier) @function)

; qualified access: the field of `Module.member` / record field access
(field_expression
  field: (identifier) @variable.other.member)
(field_expression
  field: (constructor_identifier) @constructor)

; record fields
(field_declaration name: (identifier) @variable.other.member)
(field_initializer name: (identifier) @variable.other.member)
(field_update path: (identifier) @variable.other.member)
(field_pattern name: (identifier) @variable.other.member)

; externs bind Python callables
(extern_declaration name: (identifier) @function)
(python_path (identifier) @namespace)
(extern_kwarg name: (identifier) @variable.parameter)

(measure_definition name: (identifier) @type)
(measure_factor (identifier) @type)
(effect_label) @attribute

(ce_expression
  builder: (module_identifier) @function.macro)

; ---- literals ----

(integer) @constant.numeric.integer
(float) @constant.numeric.float
(dimensionless) @constant.numeric
(boolean) @constant.builtin.boolean
(string) @string
(raw_string) @string
(fstring) @string
(string_content) @string
(escape_sequence) @constant.character.escape

(interpolation
  "{" @punctuation.special
  "}" @punctuation.special)
(debug_marker) @operator

(comment) @comment.line

; ---- keywords ----

[
  "module"
  "with"
  "as"
] @keyword

"let" @keyword.function
"fun" @keyword.function

[
  "type"
  "measure"
  "extern"
] @keyword.storage.type

"mut" @keyword.storage.modifier
"pure" @keyword.storage.modifier

"import" @keyword.control.import

[
  "if"
  "then"
  "elif"
  "else"
  "match"
  "case"
] @keyword.control.conditional

"try" @keyword.control.exception

[
  "return"
  "return!"
  "yield"
  "yield!"
] @keyword.control.return

[
  "let!"
  "do!"
] @keyword

[
  "async"
  "seq"
  "result"
] @function.macro

[
  "and"
  "or"
  "not"
] @keyword.operator

; ---- operators & punctuation ----

[
  "|>"
  "<|"
  ">>"
  "<<"
  "->"
  "<-"
  "=="
  "!="
  "<="
  ">="
  "<"
  ">"
  "+"
  "-"
  "*"
  "/"
  "//"
  "%"
  "**"
  "="
  "^"
  "|"
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  ","
  "."
  ":"
] @punctuation.delimiter
