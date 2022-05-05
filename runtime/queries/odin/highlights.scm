; Function calls

(call_expression
  function: (identifier) @function)

(call_expression
  function: (selector_expression
    field: (field_identifier) @function))


; ; Function definitions

(function_declaration
  name: (identifier) @function)

(proc_group
  (identifier) @function)

; ; Identifiers

(type_identifier) @type
(field_identifier) @variable.other.member
(identifier) @variable

(const_declaration
  (identifier) @constant)
(const_declaration_with_type
  (identifier) @constant)

"any" @type

(directive_identifier) @constant

; ; Operators

[
  "?"
  "-"
  "-="
  ":="
  "!"
  "!="
  "*"
  "*"
  "*="
  "/"
  "/="
  "&"
  "&&"
  "&="
  "%"
  "%="
  "^"
  "+"
  "+="
  "<-"
  "<"
  "<<"
  "<<="
  "<="
  "="
  "=="
  ">"
  ">="
  ">>"
  ">>="
  "|"
  "|="
  "||"
  "~"
  ".."
  "..<"
  "..="
  "::"
] @operator

; ; Keywords

[
  ; "asm"
  "auto_cast"
  ; "bit_set"
  "cast"
  ; "context"
  ; "or_else"
  ; "or_return"
  "in"
  ; "not_in"
  "distinct"
  "foreign"
  "transmute"
  ; "typeid"

  "break"
  "case"
  "continue"
  "defer"
  "else"
  "using"
  "when"
  "where"
  "fallthrough"
  "for"
  "proc"
  "if"
  "import"
  "map"
  "package"
  "return"
  "struct"
  "union"
  "enum"
  "switch"
  "dynamic"
] @keyword

; ; Literals

[
  (interpreted_string_literal)
  (raw_string_literal)
  (rune_literal)
] @string

(escape_sequence) @constant.character.escape

(int_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float
(imaginary_literal) @constant.numeric

[
  (true)
  (false)
] @constant.builtin.boolean

[
  (nil)
  (undefined)
] @constant.builtin

(comment) @comment.line
