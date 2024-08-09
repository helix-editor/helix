(package_clause "package" @keyword.control.import)

(package_identifier) @namespace

(import_declaration "import" @keyword.control.import)

[
  "!"
  "*"
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
  "+"
  "-"
  "*"
  "/"
] @operator

(unary_expression "*" @operator)

(unary_expression "=~" @operator.regexp)

(unary_expression "!~" @operator.regexp)

(binary_expression _ "&" @operator.unify _)

(binary_expression _ "|" @operator.disjunct _)

(builtin) @function.builtin

(qualified_identifier) @function.builtin

(let_clause "let" @keyword.storage.type)

(for_clause "for" @keyword.control.repeat)
(for_clause "in" @keyword.control.repeat)

(guard_clause "if" @keyword.control.conditional)

(comment) @comment

[
  (string_type)
  (simple_string_lit)
  (multiline_string_lit)
  (bytes_type)
  (simple_bytes_lit)
  (multiline_bytes_lit)
] @string

[
  (number_type)
  (int_lit)
  (int_type)
  (uint_type)
] @constant.numeric.integer

[
  (float_lit)
  (float_type)
] @constant.numeric.float

[
  (bool_type)
  (true)
  (false)
] @constant.builtin.boolean

(null) @constant.builtin

(ellipsis) @punctuation.bracket

[
  ","
  ":"
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

(interpolation "\\(" @punctuation.bracket (_) ")" @punctuation.bracket) @variable.other.member

(identifier) @variable

; Types
(primitive_type) @type.builtin

(
  (identifier) @keyword.storage.type
  (#match? @keyword.storage.type "^#")
)

[
  (slice_type)
  (pointer_type)
] @type ; In attributes


(field (label (identifier) @variable.other.member))

(field (label alias: (identifier) @label))

(attribute (identifier) @attribute)
