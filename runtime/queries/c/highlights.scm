[
  "sizeof"
] @keyword

[
  "enum"
  "struct"
  "union"
] @keyword.storage.type

[
  "const"
  "extern"
  "inline"
  "register"
  "typedef"
  "volatile"
  (storage_class_specifier)
] @keyword.storage.modifier

[
  "for"
  "do"
  "while"
  "break"
  "continue"
] @keyword.control.repeat

[
  "goto"
  "if"
  "else"
  "switch"
  "case"
  "default"
] @keyword.control.conditional

"return" @keyword.control.return

[
  "defined"
  "#define"
  "#elif"
  "#else"
  "#endif"
  "#if"
  "#ifdef"
  "#ifndef"
  "#include"
  (preproc_directive)
] @keyword.directive

[
  "+"
  "-"
  "*"
  "/"
  "++"
  "--"
  "%"
  "=="
  "!="
  ">"
  "<"
  ">="
  "<="
  "&&"
  "||"
  "!"
  "&"
  "|"
  "^"
  "~"
  "<<"
  ">>"
  "="
  "+="
  "-="
  "*="
  "/="
  "%="
  "<<="
  ">>="
  "&="
  "^="
  "|="
  "->"
  "::"
  "?"
  "..."
] @operator

["," "." ":" ";"] @punctuation.delimiter

["(" ")" "[" "]" "{" "}"] @punctuation.bracket

[(true) (false)] @constant.builtin.boolean

(enumerator) @type.enum.variant

(string_literal) @string
(system_lib_string) @string

(null) @constant
(number_literal) @constant.numeric.integer
(char_literal) @constant.character

(call_expression
  function: (identifier) @function)
(call_expression
  function: (field_expression
    field: (field_identifier) @function))
(function_declarator
  declarator: (identifier) @function)
(parameter_declaration
  declarator: (identifier) @variable.parameter)
(parameter_declaration
  (pointer_declarator
    declarator: (identifier) @variable.parameter))
(preproc_function_def
  name: (identifier) @function.special)

(field_identifier) @variable.other.member
(statement_identifier) @label
(struct_specifier) @type
(type_definition) @type
(type_identifier) @type
(primitive_type) @type.builtin
(sized_type_specifier) @type

((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]*$"))

(identifier) @variable

(comment) @comment
