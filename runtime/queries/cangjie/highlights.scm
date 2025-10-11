
(stringLiteral) @string
(booleanLiteral) @constant
(varBindingPattern) @variable
[
    (integerLiteral)
    (floatLiteral)
] @number
(modifiers) @keyword
(className) @type
(structName) @type
(interfaceName) @type
(returnType) @type
(Bool) @type
(thisSuperExpression) @variable.builtin

(funcName) @function
(propertyName) @property

[
    (blockComment)
    (lineComment)
] @comment

[
    "Int8"
    "Int16"
    "Int32"
    "Int64"
    "IntNative"
    "UInt8"
    "UInt16"
    "UInt32"
    "UInt64"
    "UIntNative"
    "Float16"
    "Float32"
    "Float64"
    "Rune"
    "Bool"
    "Unit"
    "Nothing"
;    "Thistype"
] @type

[
    "struct"
    "enum"
    "package"
    "import"
    "class"
    "interface"
    "func"
    "main"
    "let"
    "var"
    "const"
    "init"
    "super"
    "if"
    "else"
    "case"
    "try"
    "catch"
    "finally"
    "for"
    "do"
    "while"
    "throw"
    "return"
    "continue"
    "break"
    "is"
    "as"
    "in"
    "match"
    "where"
    "extend"
    "macro"
    "static"
    "public"
    "private"
    "protected"
    "internal"
    "override"
    "redef"
    "abstract"
    "open"
    "operator"
    "foreign"
    "inout"
    "prop"
    "mut"
    "unsafe"
    "get"
    "set"
    "spawn"
    "synchronized"
    "type"
] @keyword

; operetors
[
    "."
    ","
    "("
    ")"
    "["
    "]"
    "{"
    "}"
    "**"
    "*"
    "%"
    "/"
    "+"
    "-"
    "++"
    "--"
    "&&"
    "||"
    "!"
    "&"
    "|"
    "^"
    "<<"
    ">>"
    ":"
    ";"
    "="
    "+="
    "-="
    "*="
    "**="
    "/="
    "%="
    "&="
    "|="
    "^="
    "<<="
    ">>="
    "->"
    "<-"
    "=>"
    "..="
    ".."
    "@"
    "?"
    "<:"
    "<"
    ">"
    "<="
    ">="
    "!="
    "=="
    "_"
    "|>"
    "~>"
    "&&="
    "||="
] @operator
