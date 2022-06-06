(number) @number
(string) @string
(boolean) @constant.builtin
(include_path) @string

(function_call function: (identifier) @function)
(module_call name: (identifier) @function)

(identifier) @variable
(special_variable) @variable.builtin

[
  "module"
  "function"
  "for"
  "intersection_for"
  "if"
  "let"
  "assign"
  "use"
  "include"
  "each"
] @keyword

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
] @delimeter

(comment) @comment