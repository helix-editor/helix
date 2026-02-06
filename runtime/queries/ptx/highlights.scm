; highlights.scm - Syntax highlighting for PTX

; Comments
(comment) @comment

; Directives
(version_directive) @keyword
(target_directive) @keyword
(address_size_directive) @keyword
(file_directive) @keyword
(section_directive) @keyword
(visibility_directive) @keyword
(pragma_directive) @keyword

; Keywords
[
  ".global"
  ".const"
  ".param"
  ".local"
  ".shared"
  ".tex"
  ".func"
  ".entry"
] @keyword

; Types
(data_type) @type

; Instructions
(opcode) @function

; Identifiers
(identifier) @variable

; Registers
(register) @variable

; Numbers
(number) @constant.numeric.integer
(float_literal) @constant.numeric.float

; Strings
(string) @string

; Operators
[
  "+"
  "-"
  "*"
  "/"
  "%"
  "&"
  "|"
  "^"
  "&&"
  "||"
] @operator

; Labels
(label) @label