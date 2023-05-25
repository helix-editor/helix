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
  operand: (identifier) @variable
  field: (identifier) @variable.other.member)

(int_literal) @constant.numeric.integer

(attribute_declaration) @attribute
(comment) @comment
[
  (c_string_literal)
  (raw_string_literal)
  (interpreted_string_literal)
  (string_interpolation)
  (rune_literal)
] @string

(escape_sequence) @constant.character.escape

[
  (pointer_type)
  (array_type)
] @type

(const_spec name: (identifier) @constant)
(global_var_type_initializer name: (identifier) @constant)
(global_var_spec name: (identifier) @constant)
((identifier) @constant (#match? @constant "^[A-Z][A-Z\\d_]*$"))


[
  (generic_type)
  (type_identifier)
] @constructor 

(builtin_type) @type.builtin

[
 (true)
 (false)
] @constant.builtin.boolean


[
  (module_identifier)
  (import_path)
] @namespace

[
  (pseudo_comptime_identifier)
  (label_name)
] @label

[
  (identifier)
] @variable


[
  "pub"
  "assert"
  "go"
  "asm"
  "defer"
  "unsafe"
  "sql"
  (none)
] @keyword

[
  "interface"
  "enum"
  "type"
  "union"
  "struct"
  "module"
] @keyword.storage.type

[
  "static"
  "const"
  "__global"
] @keyword.storage.modifier

[
  "mut"
] @keyword.storage.modifier.mut

[
  "shared"
  "lock"
  "rlock"
  "spawn"
] @keyword.control

[
  "if"
  "select"
  "else"
  "match"
] @keyword.control.conditional

[
  "for"
] @keyword.control.repeat

[
  "goto"
  "return"
] @keyword.control.return

[
  "fn"
] @keyword.control.function


[
  "import"
] @keyword.control.import

[
  "as"
  "in"
  "is"
  "or"
] @keyword.operator

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
