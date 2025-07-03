;; Special forms
[
  "if"
  "let"
  "prog"
  "syscall"
] @keyword

;; Apply
(apply . (symbol) @function)

;; Use module
[ "use" ] @keyword

(use_module . (symbol) @namespace)

;; Function definitions
[ "def" ] @keyword

(function_definition name: (symbol) @function)
(function_definition parameters: (parameters (symbol) @variable.parameter))
(function_definition docstring: (string) @comment)

;; Macro definitions
[ "mac" ] @keyword

(macro_definition name: (symbol) @function)
(macro_definition parameters: (parameters (symbol) @variable.parameter))
(macro_definition docstring: (string) @comment)

;; Lambda 
[ "\\" ] @keyword

(lambda parameters: (parameters (symbol) @variable.parameter))

;; Atoms
(char) @constant.character
(comment) @comment
(number) @constant.numeric
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
] @constant.builtin.boolean

;; Highlight variable names used in anamorphic macros.
[
  "it"
] @variable.builtin
