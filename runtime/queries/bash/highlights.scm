[
  (string)
  (raw_string)
  (ansi_c_string)
  (heredoc_body)
] @string

[
  (heredoc_start)
  (heredoc_end)
] @label

(command_name) @function

(variable_name) @variable.other.member

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
]@embedded

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
