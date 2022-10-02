[
  "abstract"
  "alias"
  "align"
  "asm"
  "assert"
  "auto"
  "body"
  "bool"
  "break"
  "byte"
  "case"
  "cast"
  "catch"
  "cdouble"
  "cent"
  "cfloat"
  "char"
  "class"
  "const"
  "continue"
  "creal"
  "dchar"
  "debug"
  "default"
  "delegate"
  "delete"
  "deprecated"
  "do"
  "double"
  "else"
  "enum"
  "export"
  "extern"
  "final"
  "finally"
  "float"
  "for"
  "foreach"
  "foreach_reverse"
  "function"
  "goto"
  "idouble"
  "if"
  "ifloat"
  "immutable"
  "import"
  "in"
  "inout"
  "int"
  "interface"
  "invariant"
  "ireal"
  "is"
  "lazy"
  "long"
  "macro"
  "mixin"
  "module"
  "new"
  "nothrow"
  "out"
  "override"
  "package"
  "pragma"
  "private"
  "protected"
  "public"
  "pure"
  "real"
  "ref"
  "return"
  "scope"
  "shared"
  "short"
  "static"
  "struct"
  "super"
  "switch"
  "synchronized"
  "template"
  "this"
  "throw"
  "try"
  "typeid"
  "typeof"
  "ubyte"
  "ucent"
  "uint"
  "ulong"
  "union"
  "unittest"
  "ushort"
  "version"
  "void"
  "wchar"
  "while"
  "with"
] @keyword

[
  "__FILE__"
  "__FILE_FULL_PATH__"
  "__MODULE__"
  "__LINE__"
  "__FUNCTION__"
  "__PRETTY_FUNCTION__"
  "__gshared"
  "__traits"
  "__vector"
  "__parameters"
] @keyword.directive

[
  "--" 
  "-" 
  "-=" 
  "->" 
  "=" 
  "!=" 
  "*" 
  "&" 
  "&&" 
  "+" 
  "++" 
  "+=" 
  "<" 
  "==" 
  ">" 
  "||" 
  ">=" 
  "<="
] @operator

[(true) (false)] @constant.builtin.boolean

(null) @constant
(number_literal) @constant.numeric.integer
(char_literal) @constant.character

;; Class
(base_list (identifier) @type)
(property_declaration (generic_name))
(property_declaration
  type: (nullable_type) @type
  name: (identifier) @variable)
(property_declaration
  type: (predefined_type) @type
  name: (identifier) @variable)
(property_declaration
  type: (identifier) @type
  name: (identifier) @variable)

((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]*$"))

(identifier) @variable

(comment) @comment