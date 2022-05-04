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
(field_identifier) @property
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

(escape_sequence) @escape

[
  (int_literal)
  (float_literal)
  (imaginary_literal)
] @number

[
  (true)
  (false)
  (nil)
  (undefined)
] @constant

(comment) @comment
