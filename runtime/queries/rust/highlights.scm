(self) @variable.builtin
(primitive_type) @type.builtin
[
  (line_comment)
  (block_comment)
] @comment

(lifetime
  "'" @label
  (identifier) @label)
(loop_label
  (identifier) @label)

[
  "::"
  "."
  ";"
] @punctuation.delimiter
[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket
(type_arguments
  [
    "<"
    ">"
  ] @punctuation.bracket)
(type_parameters
  [
    "<"
    ">"
  ] @punctuation.bracket)

[
  "*"
  "'"
  "->"
  "=>"
  "<="
  "="
  "=="
  "!"
  "!="
  "%"
  "%="
  "&"
  "&="
  "&&"
  "|"
  "|="
  "||"
  "^"
  "^="
  "*"
  "*="
  "-"
  "-="
  "+"
  "+="
  "/"
  "/="
  ">"
  "<"
  ">="
  ">>"
  "<<"
  ">>="
  "@"
  ".."
  "..="
  "'"
] @operator

; Section - Literals
(escape_sequence) @constant.character.escape
[
  (string_literal)
  (raw_string_literal)
] @string
(char_literal) @constant.character
(boolean_literal) @constant.builtin.boolean
(integer_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float

; Section - Keywords
[
  "in"
  "await"
] @keyword.operator
[
  "if"
  "else"
  "match"
] @keyword.control.conditional
[
  "while"
  "loop"
  "break"
  "continue"
] @keyword.control.repeat
(for_expression
  "for" @keyword.control.repeat)
"return" @keyword.control.return

[
  "extern"
  "use"
] @keyword.control.import
(mod_item "mod" @keyword.control.import !body)
(use_as_clause "as" @keyword.control.import)
[
  "as"
  "where"
  "for"
  "ref"
  "move"
  "dyn"
] @keyword.operator
[
  (visibility_modifier)
  "mod"
  "struct"
  "enum"
  "impl"
  "trait"
  "type"
  "union"
  "unsafe"
  "default"
  "macro_rules!"
  "let"
  "static"
  "const"
  "async"
] @keyword
(mutable_specifier) @keyword.mut
"fn" @keyword.function

; Section - Attributes
(meta_arguments
  (meta_item
    (identifier) @type))
(meta_item
  (identifier) @tag
  value: (string_literal))
(meta_item
  (identifier) @function.macro)

; Section - Macros
(macro_definition
  name: (identifier) @function.macro)
(macro_invocation
  macro: [
    (identifier) @function.macro
    (scoped_identifier
      name: (identifier) @function.macro)
  ]
  "!" @function.macro)

; Section - Identifiers
(type_identifier) @type
(const_item
  name: (identifier) @constant)
(static_item
  name: (identifier) @constant)
(function_item
  name: (identifier) @function)
(function_signature_item
  name: (identifier) @function)

(metavariable) @variable.parameter
(let_declaration
  (identifier) @variable)
(tuple_pattern
  (identifier) @variable)
(parameter
	pattern: (identifier) @variable.parameter)
(closure_parameters
	(identifier) @variable.parameter)

(field_declaration
  name: (field_identifier) @variable.other.member)
(field_initializer
  (field_identifier) @variable.other.member)
(shorthand_field_initializer
  (identifier) @variable.other.member)
(shorthand_field_identifier) @variable.other.member
