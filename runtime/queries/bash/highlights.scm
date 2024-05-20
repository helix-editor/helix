[
  (string)
  (raw_string)
  (heredoc_body)
  (heredoc_start)
] @string

(command_name) @function

(variable_name) @variable

((variable_name) @constant
  (#match? @constant "^[A-Z][A-Z_0-9]*$"))

[
  "if"
  "then"
  "else"
  "elif"
  "fi"
  "case"
  "in"
  "esac"
] @keyword.control.conditional

[
  "for"
  "do"
  "done"
  "select"
  "until"
  "while"
] @keyword.control.repeat

[
  "declare"
  "typeset"
  "export"
  "readonly"
  "local"
  "unset"
  "unsetenv"
] @keyword

"function" @keyword.function

(comment) @comment

(function_definition name: (word) @function)

(file_descriptor) @constant.numeric.integer

[
  (command_substitution)
  (process_substitution)
  (expansion)
] @embedded

[
  "$"
  "&&"
  ">"
  ">>"
  "<"
  "|"
] @operator

(
  (command (_) @constant)
  (#match? @constant "^-")
)
