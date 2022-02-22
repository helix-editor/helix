[
  (container_doc_comment)
  (doc_comment)
  (line_comment)
] @comment

[
  variable: (IDENTIFIER)
  variable_type_function: (IDENTIFIER)
] @variable

parameter: (IDENTIFIER) @variable.parameter

[
  field_member: (IDENTIFIER)
  field_access: (IDENTIFIER)
] @variable.other.member

;; assume TitleCase is a type
(
  [
    variable_type_function: (IDENTIFIER)
    field_access: (IDENTIFIER)
    parameter: (IDENTIFIER)
  ] @type
  (#match? @type "^[A-Z]([a-z]+[A-Za-z0-9]*)*$")
)
;; assume camelCase is a function
(
  [
    variable_type_function: (IDENTIFIER)
    field_access: (IDENTIFIER)
    parameter: (IDENTIFIER)
  ] @function
  (#match? @function "^[a-z]+([A-Z][a-z0-9]*)+$")
)

;; assume all CAPS_1 is a constant
(
  [
    variable_type_function: (IDENTIFIER)
    field_access: (IDENTIFIER)
  ] @constant
  (#match? @constant "^[A-Z][A-Z_0-9]+$")
)

[
  function_call: (IDENTIFIER)
  function: (IDENTIFIER)
] @function

exception: "!" @function.macro

(
  (IDENTIFIER) @variable.builtin
  (#eq? @variable.builtin "_")
)

(PtrTypeStart "c" @variable.builtin)

(
  (ContainerDeclType
    [
      (ErrorUnionExpr)
      "enum"
    ]
  )
  (ContainerField (IDENTIFIER) @constant)
)

field_constant: (IDENTIFIER) @constant

(BUILTINIDENTIFIER) @function.builtin

((BUILTINIDENTIFIER) @keyword.control.import
  (#any-of? @keyword.control.import "@import" "@cImport"))

(INTEGER) @constant.numeric.integer

(FLOAT) @constant.numeric.float

[
  (LINESTRING)
  (STRINGLITERALSINGLE)
] @string

(CHAR_LITERAL) @constant.character
(EscapeSequence) @constant.character.escape
(FormatSequence) @string.special

[
  "allowzero"
  "volatile"
  "anytype"
  "anyframe"
  (BuildinTypeExpr)
] @type.builtin

(BreakLabel (IDENTIFIER) @label)
(BlockLabel (IDENTIFIER) @label)

[
  "true"
  "false"
] @constant.builtin.boolean

[
  "undefined"
  "unreachable"
  "null"
] @constant.builtin

[
  "else"
  "if"
  "switch"
] @keyword.control.conditional

[
  "for"
  "while"
] @keyword.control.repeat

[
  "or"
  "and"
  "orelse"
] @operator

[
  "struct"
  "enum"
  "union"
  "error"
  "packed"
  "opaque"
] @keyword

[
  "try"
  "error"
  "catch"
] @function.macro

; VarDecl
[
  "threadlocal"
  "fn"
] @keyword.function

[
  "const"
  "var"
  "test"
  "pub"
  "usingnamespace"
] @keyword

[
  "return"
  "break"
  "continue"
] @keyword.control

; Macro
[
  "defer"
  "errdefer"
  "async"
  "nosuspend"
  "await"
  "suspend"
  "resume"
  "export"
  "extern"
] @function.macro

; PrecProc
[
  "comptime"
  "inline"
  "noinline"
  "asm"
  "callconv"
  "noalias"
] @keyword.directive

[
  "linksection"
  "align"
] @function.builtin

[
  (CompareOp)
  (BitwiseOp)
  (BitShiftOp)
  (AdditionOp)
  (AssignOp)
  (MultiplyOp)
  (PrefixOp)
  "*"
  "**"
  "->"
  ".?"
  ".*"
  "?"
] @operator

[
  ";"
  "."
  ","
  ":"
] @punctuation.delimiter

[
  ".."
  "..."
] @punctuation.special

[
  "["
  "]"
  "("
  ")"
  "{"
  "}"
  (Payload "|")
  (PtrPayload "|")
  (PtrIndexPayload "|")
] @punctuation.bracket

; Error
(ERROR) @keyword
