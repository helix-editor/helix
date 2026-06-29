
(type_identifier) @type
((type_identifier) @type.builtin (#any-of? @type.builtin "int" "bool" "cell" "slice" "builder" "continuation" "tuple" "coins" "address" "void" "self"))

(identifier) @variable
((identifier) @constant
  (#match? @constant "^[A-Z][A-Z\\d_]*$"))
((identifier) @variable.builtin
 (#eq? @variable.builtin "self"))

(comment) @comment

[
  "(" ")"
  "{" "}"
] @punctuation.bracket

[
  ; "::"
  "."
  ";"
  ","
  ":"
] @punctuation.delimiter

[
  "do"
  "if"
  "as"
  "fun"
  "asm"
  "get"
  "try"
  "var"
  "val"
  "else"
  "true"
  "tolk"
  "const"
  "false"
  "throw"
  "redef"
  "while"
  "catch"
  "return"
  "assert"
  "import"
  "global"
  "repeat"
  "mutate"
  "struct"
  "type"
  "match"
  "lazy"
  (null_literal)
  (builtin_specifier)
] @keyword

[
  "="
  "+="
  "-="
  "*="
  "/="
  "%="
  "<<="
  ">>="
  "&="
  "|="
  "^="

  "=="
  "<"
  ">"
  "<="
  ">="
  "!="
  "<=>"
  "<<"
  ">>"
  "~>>"
  "^>>"
  "-"
  "+"
  "|"
  "^"
  "*"
  "/"
  "%"
  "~/"
  "^/"
  "&"
  "~"
  "."
  "!"
  "&&"
  "||"

  "->"
  "=>"
] @operator

(string_literal) @string
(number_literal) @constant.numeric
(boolean_literal) @constant.builtin.boolean

(annotation) @attribute

(function_declaration
  name: (identifier) @function)
(method_declaration
  name: (identifier) @function)
(get_method_declaration
  name: (identifier) @function)
(function_call
  callee: (identifier) @function)
(function_call
  callee: (dot_access (identifier) @type "." (identifier) @function))
(dot_access
  field: (identifier) @variable)


(struct_declaration
  "struct"
  name: (identifier) @type)
(struct_field_declaration
  name: (identifier) @variable.other.member)
(instance_argument
  name: (identifier) @variable.other.member)
