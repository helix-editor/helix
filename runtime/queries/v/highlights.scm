(parameter_declaration
  name: (identifier) @variable.parameter)
(function_declaration
  name: (identifier) @function)
(function_declaration
  receiver: (parameter_list)
  name: (identifier) @function.method)

(call_expression
  function: (identifier) @function)
(call_expression
  function: (selector_expression
    field: (identifier) @function.method))

(field_identifier) @variable.other.member
(selector_expression
  field: (identifier) @variable.other.member)

(int_literal) @constant.numeric.integer
(interpreted_string_literal) @string
(rune_literal) @string
(escape_sequence) @constant.character.escape

[
  (type_identifier)
  (builtin_type)
  (pointer_type)
  (array_type)
] @type

[
  (identifier)
  (module_identifier)
  (import_path)
] @variable

[
 "as"
 "asm"
 "assert"
 ;"atomic"
 ;"break"
 "const"
 ;"continue"
 "defer"
 "else"
 "enum"
 "fn"
 "for"
 "$for"
 "go"
 "goto"
 "if"
 "$if"
 "import"
 "in"
 "!in"
 "interface"
 "is"
 "!is"
 "lock"
 "match"
 "module"
 "mut"
 "or"
 "pub"
 "return"
 "rlock"
 "select"
 ;"shared"
 ;"static"
 "struct"
 "type"
 ;"union"
 "unsafe"
] @keyword

[
 (true)
 (false)
] @boolean

[
 "."
 ","
 ":"
 ";"
] @punctuation.delimiter

[
 "("
 ")"
 "{"
 "}"
 "["
 "]"
] @punctuation.bracket

(array) @punctuation.bracket

[
 "++"
 "--"

 "+"
 "-"
 "*"
 "/"
 "%"

 "~"
 "&"
 "|"
 "^"

 "!"
 "&&"
 "||"
 "!="

 "<<"
 ">>"

 "<"
 ">"
 "<="
 ">="

 "+="
 "-="
 "*="
 "/="
 "&="
 "|="
 "^="
 "<<="
 ">>="

 "="
 ":="
 "=="

 "?"
 "<-"
 "$"
 ".."
 "..."
] @operator

(comment) @comment