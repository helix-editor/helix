; Includes
[
  "package"
  "import"
] @keyword.control.import

; Namespaces
(package_identifier) @namespace

(import_spec
  [
    "."
    "_"
  ] @punctuation.special)

[
  (attr_path)
  (package_path)
] @string.special.url ; In attributes

; Attributes
(attribute) @attribute

; Conditionals
"if" @keyword.control.conditional

; Repeats
"for" @keyword.control.repeat

(for_clause
  "_" @punctuation.special)

; Keywords
"let" @keyword

"in" @keyword.operator

; Operators
[
  "+"
  "-"
  "*"
  "/"
  "|"
  "&"
  "||"
  "&&"
  "=="
  "!="
  "<"
  "<="
  ">"
  ">="
  "=~"
  "!~"
  "!"
  "="
] @operator

; Fields & Properties
(field
  (label
    (identifier) @variable.other.member))

(selector_expression
  (_)
  (identifier) @variable.other.member)

; Functions
(call_expression
  function: (identifier) @function)

(call_expression
  function: (selector_expression
    (_)
    (identifier) @function))

(call_expression
  function: (builtin_function) @function)

(builtin_function) @function.builtin

; Variables
(identifier) @variable

; Types
(primitive_type) @type.builtin

((identifier) @type
  (#match? @type "^_?#"))

[
  (slice_type)
  (pointer_type)
] @type ; In attributes

; Punctuation
[
  ","
  ":"
] @punctuation.delimiter

[
  "{"
  "}"
  "["
  "]"
  "("
  ")"
  "<"
  ">"
] @punctuation.bracket

[
  (ellipsis)
  "?"
] @punctuation.special

; Literals
(string) @string

[
  (escape_char)
  (escape_unicode)
] @constant.character.escape

(number) @constant.numeric

(float) @constant.numeric.float

(si_unit
  (float)
  (_) @string.special.symbol)

(boolean) @constant.builtin.boolean

[
  (null)
  (top)
  (bottom)
] @constant.builtin

; Interpolations
(interpolation
  "\\(" @punctuation.special
  (_)
  ")" @punctuation.special)

(interpolation
  "\\("
  (identifier) @variable
  ")")

; Comments
(comment) @comment
