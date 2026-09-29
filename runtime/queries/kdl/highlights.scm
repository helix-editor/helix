; Variables
(identifier) @variable

; Nodes
(node
  name: (identifier) @tag)
(node
  name: (identifier
    (string) @tag))

; Types
(type) @type
(type
  name: (identifier) @type)
; Reserved type annotations are advisory in the spec, not syntax.
((type
  name: (identifier) @type.builtin)
  (#any-of? @type.builtin
    "base64" "base85" "country-2" "country-3" "country-subdivision" "currency" "date" "date-time" "decimal" "decimal64"
    "decimal128" "duration" "email" "f32" "f64" "hostname" "i8" "i16" "i32" "i64" "i128" "idn-email" "idn-hostname"
    "ipv4" "ipv6" "irl" "irl-reference" "isize" "regex" "time" "u8" "u16" "u32" "u64" "u128" "url" "url-reference"
    "url-template" "usize" "uuid"))

; Properties
(prop
  key: (identifier) @property)

; Operators
"=" @operator

; Literals
[
  "null"
  "#null"
] @constant.builtin
(boolean) @constant.builtin.boolean
(multi_line_string) @string
(escape) @string.escape
(escaped_whitespace) @string.escape
(keyword_number) @constant.numeric
(number) @constant.numeric.integer
(number
  (decimal) @constant.numeric.float)
(number
  (exponent) @constant.numeric)
(string) @string
(string_fragment) @string

; Punctuation
[
  "{"
  "}"
] @punctuation.bracket
[
  "("
  ")"
] @punctuation.bracket
";" @punctuation.delimiter

; Comments
[
    (multi_line_comment)
    (node_children_comment)
    (node_comment)
    (node_field_comment)
    (single_line_comment)
    (version)
] @comment

