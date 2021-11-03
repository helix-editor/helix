; Keywords

[
  "alias"
  "and"
  "begin"
  "break"
  "case"
  "class"
  "def"
  "do"
  "else"
  "elsif"
  "end"
  "ensure"
  "for"
  "if"
  "in"
  "module"
  "next"
  "or"
  "rescue"
  "retry"
  "return"
  "then"
  "unless"
  "until"
  "when"
  "while"
  "yield"
] @keyword

((identifier) @keyword
 (#match? @keyword "^(private|protected|public)$"))

; Function calls

((identifier) @function.method.builtin
 (#eq? @function.method.builtin "require"))

"defined?" @function.method.builtin

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
 (#match? @constant.builtin "^__(FILE|LINE|ENCODING)__$"))

((constant) @constant
 (#match? @constant "^[A-Z\\d_]+$"))

(constant) @constructor

(self) @variable.builtin
(super) @variable.builtin

(block_parameter (identifier) @variable.parameter)
(block_parameters (identifier) @variable.parameter)
(destructured_parameter (identifier) @variable.parameter)
(hash_splat_parameter (identifier) @variable.parameter)
(lambda_parameters (identifier) @variable.parameter)
(method_parameters (identifier) @variable.parameter)
(splat_parameter (identifier) @variable.parameter)

(keyword_parameter name: (identifier) @variable.parameter)
(optional_parameter name: (identifier) @variable.parameter)

((identifier) @function.method
 (#is-not? local))
(identifier) @variable

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
  (hash_key_symbol)
  (bare_symbol)
] @string.special.symbol

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
]@constant.builtin

(interpolation
  "#{" @punctuation.special
  "}" @punctuation.special) @embedded

(comment) @comment

; Operators

[
"="
"=>"
"->"
] @operator

[
  ","
  ";"
  "."
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "%w("
  "%i("
] @punctuation.bracket
