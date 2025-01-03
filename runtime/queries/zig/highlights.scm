; Comments

((comment) @comment.block.documentation
  (#match? @comment.block.documentation "^//!"))

(comment) @comment.line

; Punctuation

[
  "["
  "]"
  "("
  ")"
  "{"
  "}"
] @punctuation.bracket

[
  ";"
  "."
  ","
  ":"
  "=>"
  "->"
] @punctuation.delimiter

(payload "|" @punctuation.bracket)

; Literals

(character) @constant.character

[
  (string)
  (multiline_string)
] @string

(integer) @constant.numeric.integer

(float) @constant.numeric.float

(boolean) @constant.builtin.boolean

(escape_sequence) @constant.character.escape

; Operator

[
  "="
  "*="
  "*%="
  "*|="
  "/="
  "%="
  "+="
  "+%="
  "+|="
  "-="
  "-%="
  "-|="
  "<<="
  "<<|="
  ">>="
  "&="
  "^="
  "|="
  "!"
  "~"
  "-"
  "-%"
  "&"
  "=="
  "!="
  ">"
  ">="
  "<="
  "<"
  "&"
  "^"
  "|"
  "<<"
  ">>"
  "<<|"
  "+"
  "++"
  "+%"
  "-%"
  "+|"
  "-|"
  "*"
  "/"
  "%"
  "**"
  "*%"
  "*|"
  "||"
  ".*"
  ".?"
  "?"
  ".."
] @operator

; Keywords

[
  "asm"
  "test"
] @keyword

[
  "error"
  "const"
  "var"
  "struct"
  "union"
  "enum"
  "opaque"
] @keyword.storage.type

; todo: keyword.coroutine
[
  "async"
  "await"
  "suspend"
  "nosuspend"
  "resume"
] @keyword

"fn" @keyword.function

[
  "and"
  "or"
  "orelse"
] @keyword.operator

[
  "try"
  "unreachable"
  "return"
] @keyword.control.return

[
  "if"
  "else"
  "switch"
  "catch"
] @keyword.control.conditional

[
  "for"
  "while"
  "break"
  "continue"
] @keyword.control.repeat

[
  "usingnamespace"
  "export"
] @keyword.control.import

[
  "defer"
  "errdefer"
] @keyword.control.exception

[
  "volatile"
  "allowzero"
  "noalias"
  "addrspace"
  "align"
  "callconv"
  "linksection"
  "pub"
  "inline"
  "noinline"
  "extern"
  "comptime"
  "packed"
  "threadlocal"
] @keyword.storage.modifier

; Builtins

[
  "c"
  "..."
] @variable.builtin

((identifier) @variable.builtin
  (#eq? @variable.builtin "_"))

(calling_convention
  (identifier) @variable.builtin)

; Modules

(variable_declaration
  (identifier) @variable ; TODO: module
  (builtin_function
    (builtin_identifier) @keyword.control.import
    (#any-of? @keyword.control.import "@import" "@cImport")))

; Functions

(call_expression
  function: (field_expression
    member: (identifier) @function.method))

(call_expression
  function: (identifier) @function)

(function_declaration
  name: (identifier) @function)

(builtin_identifier) @function.builtin

; Fields

(field_initializer
  .
  (identifier) @variable.other.member)

(field_expression
  (_)
  member: (identifier) @variable.other.member)

(container_field
  name: (identifier) @variable.other.member)

(initializer_list
  (assignment_expression
      left: (field_expression
              .
              member: (identifier) @variable.other.member)))

; Labels

(block_label (identifier) @label)

(break_label (identifier) @label)

; Constants

((identifier) @constant
  (#match? @constant "^[A-Z][A-Z_0-9]+$"))

[
  "null"
  "undefined"
] @constant.builtin

(field_expression
  .
  member: (identifier) @constant)

(enum_declaration
  (container_field
    type: (identifier) @constant))

; Types

(parameter
  type: (identifier) @type)

((identifier) @type
  (#lua-match? @type "^[A-Z_][a-zA-Z0-9_]*"))

(variable_declaration
  (identifier) @type
  "="
  [
    (struct_declaration)
    (enum_declaration)
    (union_declaration)
    (opaque_declaration)
  ])

[
  (builtin_type)
  "anyframe"
] @type.builtin

; Parameters

(parameter
  name: (identifier) @variable.parameter)

; Variables

(identifier) @variable
