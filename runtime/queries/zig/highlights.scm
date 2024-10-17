[
  (container_doc_comment)
  (doc_comment)
] @comment.documentation

[
  (line_comment)
] @comment.line

;; assume TitleCase is a type
(
  [
    variable_type_function: (IDENTIFIER)
    field_access: (IDENTIFIER)
    parameter: (IDENTIFIER)
  ] @type
  (#match? @type "^[A-Z]([a-z]+[A-Za-z0-9]*)+$")
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

;; _
(
  (IDENTIFIER) @variable.builtin
  (#eq? @variable.builtin "_")
)

;; C Pointers [*c]T
(PtrTypeStart "c" @variable.builtin)

[
  variable: (IDENTIFIER)
  variable_type_function: (IDENTIFIER)
] @variable

parameter: (IDENTIFIER) @variable.parameter

[
  field_member: (IDENTIFIER)
  field_access: (IDENTIFIER)
] @variable.other.member

[
  function_call: (IDENTIFIER)
  function: (IDENTIFIER)
] @function

exception: "!" @keyword.control.exception

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
] @keyword.operator

[
  "enum"
] @type.enum

[
  "struct"
  "union"
  "packed"
  "opaque"
  "export"
  "extern"
  "linksection"
] @keyword.storage.type

[
  "const"
  "var"
  "threadlocal"
  "allowzero"
  "volatile"
  "align"
] @keyword.storage.modifier

[
  "try"
  "error"
  "catch"
] @keyword.control.exception

[
  "fn"
] @keyword.function

[
  "test"
] @keyword

[
  "pub"
  "usingnamespace"
] @keyword.control.import

[
  "return"
  "break"
  "continue"
] @keyword.control.return

[
  "defer"
  "errdefer"
  "async"
  "nosuspend"
  "await"
  "suspend"
  "resume"
] @function.macro

[
  "comptime"
  "inline"
  "noinline"
  "asm"
  "callconv"
  "noalias"
] @keyword.directive

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

(ERROR) @keyword.control.exception
