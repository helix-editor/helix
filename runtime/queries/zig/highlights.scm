[
  (container_doc_comment)
  (doc_comment)
  (line_comment)
] @comment

;field in top level decl, and in struct, union...
(ContainerField
  (IDENTIFIER) @property
  (SuffixExpr (IDENTIFIER) @type)?
)

; INFO: field become a function if type is a function?
; const u = union { this_is_function: fn () void };
(ContainerField
  (IDENTIFIER) @function
  (SuffixExpr (FnProto))
)

;enum and tag union field is constant
(
  [
    ; union(Tag){}
    (ContainerDeclType (SuffixExpr (IDENTIFIER) @type))

    ; enum{}
    (ContainerDeclType "enum")
  ]
  (ContainerField (IDENTIFIER) @constant)?
)

; INFO: .IDENTIFIER is a field?
(SuffixExpr 
  "."
  (IDENTIFIER) @property
)

; error.OutOfMemory;
(SuffixExpr 
  "error"
  "."
  (IDENTIFIER) @constant
)

(VarDecl
  (IDENTIFIER) @type
  [
    ; const IDENTIFIER = struct/enum/union...
    (SuffixExpr (ContainerDecl))

    ; const A = u8;
    (SuffixExpr (BuildinTypeExpr))
  ]
)

; const fn_no_comma = fn (i32, i32) void;
(VarDecl
  (IDENTIFIER) @function
  (SuffixExpr (FnProto))
)

; var x: IDENTIFIER
type: (SuffixExpr (IDENTIFIER) @type)

; IDENTIFIER{}
constructor: (SuffixExpr (IDENTIFIER) @constructor)

;{.IDENTIFIER = 1}
(FieldInit (IDENTIFIER) @property)

; var.field
(SuffixOp (IDENTIFIER) @property)

; var.func().func().field
( 
  (SuffixOp
    (IDENTIFIER) @function
  )
  .
  (FnCallArguments)
)
; func()
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

(ParamDecl 
  (ParamType (SuffixExpr (IDENTIFIER) @variable.parameter))
)

(ParamDecl 
  (IDENTIFIER) @variable.parameter
  ":"
  [
    (ParamType (SuffixExpr (IDENTIFIER) @type))
    (ParamType)
  ]
)

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
