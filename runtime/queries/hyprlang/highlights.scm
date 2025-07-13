(comment) @comment

[
  "source"
  "exec"
  "exec-once"
] @function.builtin

(keyword
  (name) @keyword)

(assignment
  (name) @variable.other.member)

(section
  (name) @namespace)

(section
  device: (device_name) @type)

(variable) @variable

"$" @punctuation.special

(boolean) @constant.builtin.boolean

(string) @string

(mod) @constant

[
  "rgb"
  "rgba"
] @function.builtin

[
  (number)
  (legacy_hex)
  (angle)
  (hex)
] @constant.numeric

"deg" @type

"," @punctuation.delimiter

[
  "("
  ")"
  "{"
  "}"
] @punctuation.bracket

[
  "="
  "-"
  "+"
] @operator
