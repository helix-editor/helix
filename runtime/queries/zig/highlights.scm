[
  (container_doc_comment)
  (doc_comment)
  (line_comment)
] @comment

; field in top level decl, and in struct, union...
(ContainerField
  (IDENTIFIER) @property
  (SuffixExpr (IDENTIFIER) @type)?
)

; error.OutOfMemory;
(SuffixExpr
  "error"
  "."
  (IDENTIFIER) @constant
)

; var x: IDENTIFIER
type: (SuffixExpr (IDENTIFIER) @type)

; IDENTIFIER{}
constructor: (SuffixExpr (IDENTIFIER) @constructor)

; fields
(FieldInit (IDENTIFIER) @property)

; foo.bar.baz.function() calls
(
  (SuffixOp
    (IDENTIFIER) @function
  )
  .
  (FnCallArguments)
)

; function() calls
(
  (
    (IDENTIFIER) @function
  )
  .
  (FnCallArguments)
)

; functionn decl
(FnProto
  (IDENTIFIER) @function
  (SuffixExpr (IDENTIFIER) @type)?
  ("!")? @function.macro
)

; function parameters and types
(ParamDecl
  (IDENTIFIER) @variable.parameter
  ":"
  [
    (ParamType (SuffixExpr (IDENTIFIER) @type))
    (ParamType)
  ]
)

; switch
(SwitchItem
  (SuffixExpr
    "."
    .
    (IDENTIFIER) @constant
  )
)

(INTEGER) @number

(FLOAT) @number

[
  (STRINGLITERAL)
  (STRINGLITERALSINGLE)
] @string

(CHAR_LITERAL) @string

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
  "undefined"
  "unreachable"
  "null"
] @constant.builtin

[
  "else"
  "if"
  "switch"
  "for"
  "while"
  "return"
  "break"
  "continue"
  "defer"
  "errdefer"
  "async"
  "nosuspend"
  "await"
  "suspend"
  "resume"
  "try"
  "catch"
] @keyword.control

[
  "struct"
  "enum"
  "union"
  "error"
  "packed"
  "opaque"
  "test"
  "usingnamespace"
  "export"
  "extern"
  "const"
  "var"
  "comptime"
  "threadlocal"
] @keyword

[
  "pub"
  "fn"
] @keyword.function

; PrecProc
[
  "inline"
  "noinline"
  "asm"
  "callconv"
  "noalias"
] @attribute

[
  (BUILTINIDENTIFIER)
  "linksection"
  "align"
] @function.builtin

[
  (CompareOp)
  (BitwiseOp)
  (BitShiftOp)
  (AdditionOp)
  (MultiplyOp)
  (PrefixOp)
  "or"
  "and"
  "orelse"
  "*"
  "**"
  "->"
  "=>"
  ".?"
  ".*"
  "="
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
  "["
  "]"
  "("
  ")"
  "{"
  "}"
  (Payload "|")
  (PtrPayload "|")
  (PtrIndexPayload "|")
] @punctuation
