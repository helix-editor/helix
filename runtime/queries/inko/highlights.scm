; Brackets and operators
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

[
  "!="
  "%"
  "%="
  "&"
  "&="
  "*"
  "**"
  "**="
  "*="
  "+"
  "+="
  "-"
  "-="
  "/"
  "/="
  "<"
  "<<"
  "<<="
  "<="
  "<="
  "=="
  ">"
  ">="
  ">="
  ">>"
  ">>="
  ">>>"
  ">>>="
  "^"
  "^="
  "|"
  "|="
] @operator

; Identifiers/variable references
(identifier) @variable

((identifier) @function
  (#is-not? local))

; Keywords
[
  "as"
  "for"
  "impl"
  "let"
  "mut"
  "ref"
  "uni"
  "move"
  "recover"
] @keyword

"fn" @keyword.function

"import" @keyword.control.import

[
  "and"
  "or"
] @keyword.operator

[
  "type"
  "trait"
] @keyword.storage.type

[
  "extern"
  (modifier)
  (visibility)
] @keyword.storage.modifier

[
  "loop"
  "while"
  (break)
  (next)
] @keyword.control.repeat

"return" @keyword.control.return

[
  "throw"
  "try"
] @keyword.control.exception

[
  "case"
  "else"
  "if"
  "match"
] @keyword.control.conditional

; Comments
(line_comment) @comment.line

; Literals
(self) @variable.builtin

(nil) @constant.builtin

[
  (true)
  (false)
] @constant.builtin.boolean

(integer) @constant.numeric.integer

(float) @constant.numeric.float

(string) @string

(escape_sequence) @constant.character.escape

(interpolation
  "${" @punctuation.special
  "}" @punctuation.special)

(constant) @constant

; Patterns
(integer_pattern) @constant.numeric.integer

(string_pattern) @string

(constant_pattern) @constant

; Types
(generic_type
  name: _ @type)

(type) @type

; Imports
(extern_import
  path: _ @string)

; Classes
(class
  name: _ @type)

(define_field
  name: _ @variable.other.member)

; Traits
(trait
  name: _ @type)

; Implementations
(implement_trait
  class: _ @type)

(reopen_class
  name: _ @type)

(bound
  name: _ @type)

; Methods
(method
  name: _ @function)

(external_function
  name: _ @function)

(argument
  name: _ @variable.parameter)

(named_argument
  name: _ @variable.parameter)

(call
  name: _ @function)

(field) @variable.other.member
