; tree-sitter-awk v0.5.1

; Order matters

[
  ";"
  ","
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @operator

(piped_io_statement [
  "|"
  "|&"
] @operator)

(redirected_io_statement [
  ">"
  ">>"
] @operator)

(update_exp [
  "++"
  "--"
] @operator)

(ternary_exp [
  "?"
  ":"
] @operator)

(assignment_exp [
  "="
  "+="
  "-="
  "*="
  "/="
  "%="
  "^="
] @operator)

(unary_exp [
  "!"
  "+"
  "-"
] @operator)

(binary_exp [
  "^"
  "**"
  "*"
  "/"
  "%"
  "+"
  "-"
  "<"
  ">"
  "<="
  ">="
  "=="
  "!="
  "~"
  "!~"
  "in"
  "&&"
  "||"
] @operator)

[
  "@include"
  "@load"
  "@namespace"
  (pattern)
] @namespace

[
  "function"
  "func"
  "print"
  "printf"
  "if"
  "else"
  "do"
  "while"
  "for"
  "in"
  "delete"
  "return"
  "exit"
  "switch"
  "case"
  "default"
  (break_statement)
  (continue_statement)
  (next_statement)
  (nextfile_statement)
  (getline_input)
  (getline_file)
] @keyword

(comment) @comment
(regex) @string.regexp
(number) @constant.numeric
(string) @string

(func_call name: (identifier) @function)
(func_def name: (identifier) @function)

(field_ref (_) @variable)
[
  (identifier)
  (field_ref)
] @variable

(ns_qualified_name "::" @operator)
(ns_qualified_name (namespace) @namespace)
