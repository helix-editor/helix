(comment) @comment

(module_clause
 (identifier) @namespace)

(import_path
 (import_name) @namespace)

(import_alias
 (import_name) @namespace)

(enum_fetch
 (reference_expression) @constant)

(enum_field_definition
 (identifier) @constant)

(global_var_definition
 (identifier) @constant)

(compile_time_if_expression
 condition: (reference_expression) @constant)

(compile_time_if_expression
 condition: (binary_expression
              left: (reference_expression) @constant
              right: (reference_expression) @constant))

(compile_time_if_expression
 condition: (binary_expression
              left: (reference_expression) @constant
              right: (unary_expression (reference_expression) @constant)))

(label_reference) @label

(parameter_declaration
 name: (identifier) @variable.parameter)
(receiver
 name: (identifier) @variable.parameter)
(function_declaration
 name: (identifier) @function)
(function_declaration
 receiver: (receiver)
 name: (identifier) @function.method)
(interface_method_definition
 name: (identifier) @function.method)

(call_expression
 name: (selector_expression
  field: (reference_expression) @function.method))

(call_expression
 name: (reference_expression) @function)

(struct_declaration
 name: (identifier) @type)

(enum_declaration
 name: (identifier) @type)

(interface_declaration
 name: (identifier) @type)

(type_declaration
 name: (identifier) @type)

(struct_field_declaration
 name: (identifier) @variable.other.member)

(field_name) @variable.other.member

(selector_expression
 field: (reference_expression) @variable.other.member)

(int_literal) @constant.numeric.integer
(escape_sequence) @constant.character.escape

[
 (c_string_literal)
 (raw_string_literal)
 (interpreted_string_literal)
 (string_interpolation)
 (rune_literal)
] @string

(string_interpolation
 (braced_interpolation_opening) @punctuation.bracket
 (interpolated_expression) @embedded
 (braced_interpolation_closing) @punctuation.bracket)

(attribute) @attribute

[
 (type_reference_expression)
 ] @type

[
 (true)
 (false)
] @constant.builtin.boolean

[
  "pub"
  "assert"
  "asm"
  "defer"
  "unsafe"
  "sql"
  (nil)
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
  "break"
  "continue"
  "go"
] @keyword.control

[
  "if"
  "$if"
  "select"
  "else"
  "$else"
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

(array_creation) @punctuation.bracket

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
