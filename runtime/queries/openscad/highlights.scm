(number) @constant.numeric
(string) @string
(boolean) @constant.builtin.boolean
(include_path) @string.special.path

(identifier) @variable

(parameters_declaration (identifier) @variable.parameter)
(function_declaration name: (identifier) @function)

(function_call function: (identifier) @function)
(module_call name: (identifier) @function)

(special_variable) @variable.builtin

[
  "function"
  "let"
  "assign"
] @keyword

[
  "for"
  "each"
  "intersection_for"
] @keyword.control.repeat

[
  "if"
] @keyword.control.conditional

[
  "module"
  "use"
  "include"
] @keyword.control.import

[
  "||"
  "&&"
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  "+"
  "-"
  "*"
  "/"
  "%"
  "^"
  "?"
  "!"
  ":"
] @operator

[
  ";"
  ","
  "."
] @punctuation.delimiter

(comment) @comment
