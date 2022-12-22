(int_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float
(bool_literal) @constant.builtin.boolean

[
  "bitcast"
  "discard"
  "enable"
  "fallthrough"
] @keyword

[
  "let"
  "override"
  "struct"
  "type"
  "var"
  (texel_format)
] @keyword.storage.type

[
  (access_mode)
  (address_space)
] @keyword.storage.modifier

"fn" @keyword.function

"return" @keyword.control.return

["," "." ":" ";"] @punctuation.delimiter

["(" ")" "[" "]" "{" "}"] @punctuation.bracket

(type_declaration ["<" ">"] @punctuation.bracket)

[
  "break"
  "continue"
  "continuing"
] @keyword.control

[
  "loop"
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

[
  "!"
  "!="
  "%"
  "%="
  "&"
  "&&"
  "&="
  "*"
  "*="
  "+"
  "++"
  "+="
  "-"
  "--"
  "-="
  "->"
  "/"
  "/="
  "<"
  "<<"
  "<="
  "="
  "=="
  ">"
  ">="
  ">>"
  "@"
  "^"
  "^="
  "|"
  "|="
  "||"
  "~"
] @operator

(function_declaration
  (identifier) @function)

(parameter
  (variable_identifier_declaration
    (identifier) @variable.parameter))

(struct_declaration
  (identifier) @type)

(struct_declaration
  (struct_member
    (variable_identifier_declaration
      (identifier) @variable.other.member)))

(type_constructor_or_function_call_expression
  (type_declaration (identifier) @function))

(type_declaration _ @type)

(attribute
  (identifier) @attribute)

(identifier) @variable

(comment) @comment
