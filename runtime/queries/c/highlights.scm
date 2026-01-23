
(identifier) @variable

((identifier) @constant
  (#match? @constant "^[A-Z][A-Z\\d_]*$"))

"sizeof" @keyword

[
  "enum"
  "struct"
  "typedef"
  "union"
] @keyword.storage.type

[
  (type_qualifier)
  (storage_class_specifier)
] @keyword.storage.modifier

[
  "goto"
  "break"
  "continue"
] @keyword.control

[
  "do"
  "for"
  "while"
] @keyword.control.repeat

[
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

"..." @punctuation

["," "." ":" "::" ";" "->"] @punctuation.delimiter

["(" ")" "[" "]" "{" "}" "[[" "]]"] @punctuation.bracket

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
  "?"
] @operator

(conditional_expression ":" @operator) ; After punctuation

(pointer_declarator "*" @type.builtin) ; After Operators
(abstract_pointer_declarator "*" @type.builtin)


[(true) (false)] @constant.builtin.boolean

(enumerator name: (identifier) @type.enum.variant)

(string_literal) @string
(system_lib_string) @string

(null) @constant
(number_literal) @constant.numeric
(char_literal) @constant.character
(escape_sequence) @constant.character.escape

(field_identifier) @variable.other.member
(statement_identifier) @label
(type_identifier) @type
(primitive_type) @type.builtin
(sized_type_specifier) @type.builtin

(call_expression
  function: (identifier) @function)
(call_expression
  function: (field_expression
    field: (field_identifier) @function))
(call_expression (argument_list (identifier) @variable))
(function_declarator
  declarator: [(identifier) (field_identifier)] @function)

; Up to 6 layers of declarators
(parameter_declaration
  declarator: (identifier) @variable.parameter)
(parameter_declaration
  (_
    (identifier) @variable.parameter))
(parameter_declaration
  (_
    (_
      (identifier) @variable.parameter)))
(parameter_declaration
  (_
    (_
      (_
        (identifier) @variable.parameter))))
(parameter_declaration
  (_
    (_
      (_
        (_
          (identifier) @variable.parameter)))))
(parameter_declaration
  (_
    (_
      (_
        (_
          (_
            (identifier) @variable.parameter))))))

(preproc_function_def
  name: (identifier) @function.special)

(attribute
  name: (identifier) @attribute)

(comment) @comment
