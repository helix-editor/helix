;; Special forms
[
  "cond"
  "if"
  "let"
  "match"
  "prog"
  "quote"
  "syscall"
  "unless"
] @keyword

;; Apply
(apply . (symbol) @function)


;; Load module
[ "load" ] @keyword

(load_module . (symbol) @namespace)

;; Function definitions
[ "def" ] @keyword

(function_definition name: (symbol) @function)
(function_definition parameters: (parameters (symbol) @variable.parameter))
(function_definition docstring: (string) @comment)

;; Lambda 
[ "\\" ] @keyword

(lambda parameters: (parameters (symbol) @variable.parameter))

;; Atoms
(char) @constant.character
(comment) @comment
(number) @number
(string) @string

;; Punctuation
[
  "("
  ")"
] @punctuation.bracket

;; Operators
(dot_item) @operator
(dot_statement) @operator
(tilde) @operator
(backquote) @operator
(quote) @operator
(unquote) @operator

;; Highlight nil and t as constants, unlike other symbols
[
  "nil"
  "t"
] @constant.builtin

