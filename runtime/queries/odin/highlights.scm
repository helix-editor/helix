[
  (calling_convention)
  (tag)
] @keyword.directive

[
  "import"
  "package"
] @namespace

[
  "foreign"
  "using"
  "struct"
  "enum"
  "union"
  "defer"
  "cast"
  "transmute"
  "auto_cast"
  "map"
  "bit_set"
  "matrix"
  "bit_field"
] @keyword

[
  "proc"
] @keyword.function

[
  "return"
  "or_return"
] @keyword.control.return

[
  "distinct"
  "dynamic"
] @keyword.storage.modifier

[
  "if"
  "else"
  "when"
  "switch"
  "case"
  "where"
  "break"
  (fallthrough_statement)
] @keyword.control.conditional

((ternary_expression
  [
    "?"
    ":"
    "if"
    "else"
    "when"
  ] @keyword.control.conditional))

[
  "for"
  "do"
  "continue"
] @keyword.control.repeat

[
  ":="
  "="
  "+"
  "-"
  "*"
  "/"
  "%"
  "%%"
  ">"
  ">="
  "<"
  "<="
  "=="
  "!="
  "~="
  "|"
  "~"
  "&"
  "&~"
  "<<"
  ">>"
  "||"
  "&&"
  "!"
  "^"
  ".."
  "+="
  "-="
  "*="
  "/="
  "%="
  "&="
  "|="
  "^="
  "<<="
  ">>="
  "||="
  "&&="
  "&~="
  "..="
  "..<"
  "?"
] @operator

[
  "or_else"
  "in"
  "not_in"
] @keyword.operator

[ "{" "}" ] @punctuation.bracket

[ "(" ")" ] @punctuation.bracket

[ "[" "]" ] @punctuation.bracket

[
  "::"
  "->"
  "."
  ","
  ":"
  ";"
] @punctuation.delimiter

[
  "@"
  "$"
] @punctuation.special

(number) @constant.numeric

(float) @constant.numeric.float

(string) @string

(character) @string

(escape_sequence) @string.special

(boolean) @constant.builtin.boolean

[
  (uninitialized)
  (nil)
] @constant.builtin

((identifier) @variable.builtin
  (#any-of? @variable.builtin "context" "self"))

(((identifier) @type.builtin)
  (#any-of? @type.builtin
    "bool" "byte" "b8" "b16" "b32" "b64"
    "int" "i8" "i16" "i32" "i64" "i128"
    "uint" "u8" "u16" "u32" "u64" "u128" "uintptr"
    "i16le" "i32le" "i64le" "i128le" "u16le" "u32le" "u64le" "u128le"
    "i16be" "i32be" "i64be" "i128be" "u16be" "u32be" "u64be" "u128be"
    "float" "double" "f16" "f32" "f64" "f16le" "f32le" "f64le" "f16be" "f32be" "f64be"
    "complex32" "complex64" "complex128" "complex_float" "complex_double"
    "quaternion64" "quaternion128" "quaternion256"
    "rune" "string" "cstring" "rawptr" "typeid" "any"))

"..." @type.builtin

[
  (comment)
  (block_comment)
] @comment

; Functions

(procedure_declaration (identifier) @function)

(procedure_declaration (identifier) @function (procedure (block)))

(procedure_declaration (identifier) @function (procedure (uninitialized)))

(overloaded_procedure_declaration (identifier) @function)

(call_expression function: (identifier) @function)

; Types

(struct_declaration (identifier) @type "::")

(enum_declaration (identifier) @type "::")

(union_declaration (identifier) @type "::")

(bit_field_declaration (identifier) @type "::")

(const_declaration (identifier) @type "::" [(array_type) (distinct_type) (bit_set_type) (pointer_type)])

(struct . (identifier) @type)

(field_type . (identifier) "." (identifier) @type)

(bit_set_type (identifier) @type ";")

(polymorphic_parameters (identifier) @type)

((identifier) @type
  (#match? @type "^[A-Z][a-z0-9_]+"))

(type (identifier) @type)

; Constants

(member_expression . "." (identifier) @constant)

(enum_declaration "{" (identifier) @constant)

((identifier) @constant
  (#match? @constant "^[A-Z0-9_]*$"))

; Attributes

(attribute (identifier) @attribute "="?)

; Labels

(label_statement (identifier) @label ":")

; Fields

(member_expression "." (identifier) @variable.other.member)

(struct_type "{" (identifier) @variable.other.member)

(struct_field (identifier) @variable.other.member "="?)

(bit_field_declaration (identifier) @variable.other.member)

(field (identifier) @variable.other.member)

; Namespaces

(package_declaration (identifier) @namespace)

(foreign_block (identifier) @namespace)

(using_statement (identifier) @namespace)

; Parameters

(parameter (identifier) @variable.parameter ":" "="? (identifier)? @constant)

(default_parameter (identifier) @variable.parameter ":=")

(named_type (identifier) @variable.parameter)

(call_expression argument: (identifier) @variable.parameter "=")

(procedure_type (parameters (parameter (identifier) @variable.parameter)))

; Variables

(identifier) @variable
