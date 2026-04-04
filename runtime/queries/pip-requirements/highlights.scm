(comment) @comment

(requirement (package) @variable)
(extras (package) @variable.parameter)

; "==" | ">" | "<" | ">=" | "<="
(version_cmp) @operator

(version) @constant.numeric

(marker_var) @attribute

(marker_op) @keyword.operator

[
  "[" "]"
  "(" ")"
] @punctuation.bracket

[
  ","
  ";"
  "@"
] @punctuation.delimiter

[
  "${" "}"
] @punctuation.special

"=" @operator

(path) @string.special.path
(url) @string.special.url

(option) @function

(env_var) @constant

(quoted_string) @string

(linebreak) @constant.character.escape
