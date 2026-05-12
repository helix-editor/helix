; Comments
(comment) @comment

; Keywords
[
  "global"
  "import"
  "private"
] @constant.builtin

[
  "rule"
] @function

[
  "meta"
  "strings"
  "condition"
] @attribute

; Operators
[
  "matches"
  "contains"
  "icontains"
  "imatches"
  "startswith"
  "istartswith"
  "endswith"
  "iendswith"
  "and"
  "or"
  "not"
  "=="
  "!="
  "<"
  ">"
  ">="
  "<="
  "of"
  "for"
  "all"
  "any"
  "none"
  "in"
] @string.special

; String modifiers
[
  "wide"
  "ascii"
  "nocase"
  "fullword"
  "xor"
  "base64"
  "base64wide"
] @keyword.storage.modifier

; Numbers and sizes
(integer_literal) @constant.numeric
(size_unit) @constant.numeric

; Strings
(double_quoted_string) @string
(single_quoted_string) @string
(escape_sequence) @constant.character.escape

; Hex strings
(hex_string) @string.special
(hex_byte) @constant.numeric
(hex_wildcard) @constant.builtin
(hex_jump) @constant.numeric

; Regular expressions
(regex_string) @string.regexp
(pattern) @string.regexp

; Boolean literals
[
  "true"
  "false"
] @constant.builtin.boolean

; Keywords and special identifiers
[
  "them"
  "all"
  "any"
  "none"
] @keyword.operator


; String identifiers
"$" @string.special.symbol
(identifier) @string
(string_identifier) @string.special.symbol

; Built-ins
[
  (filesize_keyword)
  (entrypoint_keyword)
] @constant.builtin

; Tags
(tag_list
  [(identifier) (tag)] @tag)

; Punctuation and delimiters
[
  "="
  ":"
  "{"
  "}"
  "["
  "]"
  "("
  ")"
  "#"
  "@"
  ".."
  "|"
  ","
  "!"
  "/"
  "\""
  "'"
  "*"
] @string.special.symbol

; Rule names
(rule_definition
  name: (identifier) @string.special)

; Meta definitions
(meta_definition
  key: (identifier) @string.special)
