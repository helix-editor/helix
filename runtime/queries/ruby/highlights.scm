; Keywords

[
  "BEGIN"
  "END"
  "alias"
  "begin"
  "class"
  "do"
  "end"
  "module"
  "in"
  "rescue"
  "ensure"
] @keyword

[
  "if"
  "else"
  "elsif"
  "when"
  "case"
  "unless"
  "then"
] @keyword.control.conditional

[
  "for"
  "while"
  "retry"
  "until"
  "redo"
] @keyword.control.repeat

[
  "yield"
  "return"
  "next"
  "break"
] @keyword.control.return

[
  "def"
  "undef"
] @keyword.function

((identifier) @keyword.control.import
 (#match? @keyword.control.import "^(require|require_relative|load|autoload)$"))

[
  "or"
  "and"
  "not"
] @keyword.operator

((identifier) @keyword.control.exception
 (#match? @keyword.control.exception "^(raise|fail)$"))

; Function calls

((identifier) @function.builtin
 (#match? @function.builtin "^(attr|attr_accessor|attr_reader|attr_writer|include|prepend|refine|private|protected|public)$"))

"defined?" @function.builtin

(call
  method: [(identifier) (constant)] @function.method)

; Function definitions

(alias (identifier) @function.method)
(setter (identifier) @function.method)
(method name: [(identifier) (constant)] @function.method)
(singleton_method name: [(identifier) (constant)] @function.method)

; Identifiers

[
  (class_variable)
  (instance_variable)
] @variable.other.member

((identifier) @constant.builtin
 (#match? @constant.builtin "^(__FILE__|__LINE__|__ENCODING__)$"))

((constant) @constant.builtin
 (#match? @constant.builtin "^(ENV|ARGV|ARGF|RUBY_PLATFORM|RUBY_RELEASE_DATE|RUBY_VERSION|STDERR|STDIN|STDOUT|TOPLEVEL_BINDING)$"))

((constant) @constant
 (#match? @constant "^[A-Z\\d_]+$"))

(constant) @constructor

(self) @variable.builtin
(super) @function.builtin

[(forward_parameter)(forward_argument)] @variable.parameter
(keyword_parameter name:((_)":" @variable.parameter) @variable.parameter)
(optional_parameter name:((_)"=" @operator) @variable.parameter)
(optional_parameter name: (identifier) @variable.parameter)
(splat_parameter name: (identifier) @variable.parameter) @variable.parameter
(hash_splat_parameter name: (identifier) @variable.parameter) @variable.parameter
(method_parameters (identifier) @variable.parameter)
(block_parameter (identifier) @variable.parameter)
(block_parameters (identifier) @variable.parameter)

((identifier) @function.method
 (#is-not? local))
[
  (identifier)
] @variable

; Literals

[
  (string)
  (bare_string)
  (subshell)
  (heredoc_body)
  (heredoc_beginning)
] @string

[
  (simple_symbol)
  (delimited_symbol)
  (bare_symbol)
] @string.special.symbol

(pair key: ((_)":" @string.special.symbol) @string.special.symbol)

(regex) @string.regexp
(escape_sequence) @constant.character.escape

[
  (integer)
  (float)
] @constant.numeric.integer

[
  (nil)
  (true)
  (false)
] @constant.builtin

(interpolation
  "#{" @punctuation.special
  "}" @punctuation.special) @embedded

(comment) @comment

; Operators
[
":"
"?"
"~"
"=>"
"->"
"!"
] @operator

(assignment
  "=" @operator)

(operator_assignment
  operator: ["+=" "-=" "*=" "**=" "/=" "||=" "|=" "&&=" "&=" "%=" ">>=" "<<=" "^="] @operator)

(binary
  operator: ["/" "|" "==" "===" "||" "&&" ">>" "<<" "<" ">" "<=" ">=" "&" "^" "!~" "=~" "<=>" "**" "*" "!=" "%" "-" "+"] @operator)

(range
  operator: [".." "..."] @operator)

[
  ","
  ";"
  "."
  "&."
] @punctuation.delimiter

[
  "|"
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "%w("
  "%i("
] @punctuation.bracket
