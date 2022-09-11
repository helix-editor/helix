(storage_class_specifier) @keyword.storage

[
  "const"
  "default"
  "enum"
  "extern"
  "inline"
  "static"
  "struct"
  "typedef"
  "union"
  "volatile"
  "goto"
  "register"
] @keyword

"sizeof" @keyword.operator
"return" @keyword.control.return

[
  "while"
  "for"
  "do"
  "continue"
  "break"
] @keyword.control.repeat

[
  "if"
  "else"
  "case"
  "switch"
] @keyword.control.conditional
(conditional_expression [ "?" ":" ] @keyword.control.conditional)

[
  "#define"
  "#include"
  "#if"
  "#ifdef"
  "#ifndef"
  "#else"
  "#elif"
  "#endif"
  (preproc_directive)
] @keyword.directive

[ ";" ":" "," ] @punctuation.delimiter
[ "(" ")" "[" "]" "{" "}"] @punctuation.bracket
"..." @punctuation.special

[
  "="

  "-"
  "*"
  "/"
  "+"
  "%"

  "~"
  "|"
  "&"
  "^"
  "<<"
  ">>"

  "->"
  "."

  "<"
  "<="
  ">="
  ">"
  "=="
  "!="

  "!"
  "&&"
  "||"

  "-="
  "+="
  "*="
  "/="
  "%="
  "|="
  "&="
  "^="
  ">>="
  "<<="
  "--"
  "++"
] @operator

;; Make sure the comma operator is given a highlight group after the comma
;; punctuator so the operator is highlighted properly.
(comma_expression [ "," ] @operator)

[
  (true)
  (false)
] @constant.builtin.boolean

(string_literal) @string
(system_lib_string) @string.special.path

(escape_sequence) @constant.character.escape
(char_literal) @constant.character
(number_literal) @constant.numeric
(null) @constant.builtin

[
  (preproc_arg)
  (preproc_defined)
] @function.macro

(statement_identifier) @label

[
 (type_identifier)
 (sized_type_specifier)
 (type_descriptor)
] @type
(primitive_type) @type.builtin

(sizeof_expression value: (parenthesized_expression (identifier) @type))
(enumerator
  name: (identifier) @type.enum.variant)

((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]*$"))

(case_statement
  value: (identifier) @constant)

;; Preproc def / undef
(preproc_def
  name: (_) @constant)
(preproc_call
  directive: (preproc_directive) @_u
  argument: (_) @constant
  (#eq? @_u "#undef"))

(call_expression
  function: (identifier) @function)
(call_expression
  function: (field_expression
    field: (field_identifier) @function))
(function_declarator
  declarator: (identifier) @function)
(preproc_function_def
  name: (identifier) @function.macro)

(comment) @comment

;; Parameters
(parameter_declaration
  declarator: (identifier) @variable.parameter)
(parameter_declaration
  declarator: (pointer_declarator) @variable.parameter)
(preproc_params (identifier) @variable.parameter)

[
  "__attribute__"
  "__cdecl"
  "__clrcall"
  "__stdcall"
  "__fastcall"
  "__thiscall"
  "__vectorcall"
  "_unaligned"
  "__unaligned"
  "__declspec"
  (attribute_declaration)
] @attribute

(field_identifier) @variable.other.member
