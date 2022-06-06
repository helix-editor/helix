(number) @constant.numeric
(string) @string
(boolean) @constant.builtin.boolean
(include_path) @string.special.path

(function_call function: (identifier) @function)
(module_call name: (identifier) @function)

(identifier) @variable
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