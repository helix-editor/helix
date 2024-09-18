(line_comment) @comment.line
(block_comment) @comment.block

(argument name: (identifier) @variable.parameter)

(local_var name: (identifier) @variable)
(environment_var name:(identifier) @variable.builtin)
(builtin_var) @constant.builtin

(function_definition name: (variable) @function)

(named_argument name: (identifier) @variable.other.member)

(method_call name: (method_name) @function.method)

(class) @keyword.storage.type

(number) @constant.numeric
(float) @constant.numeric.float

(string) @string
(symbol) @string.special.symbol

[
"&&"
"||"
"&"
"|"
"^"
"=="
"!="
"<"
"<="
">"
">="
"<<"
">>"
"+"
"-"
"*"
"/"
"%"
"="
"|@|"
"@@"
"@|@"
] @operator

[
"arg"
"classvar"
"const"
"var"
] @keyword

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "|"
] @punctuation.bracket

[
  ";"
  "."
  ","
] @punctuation.delimiter

(control_structure) @keyword.control.conditional

(escape_sequence) @string.special

(duplicated_statement) @keyword.control.repeat
