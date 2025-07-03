;; Keywords
[
  "if"
  "prog"
  "syscall"
] @keyword

;; Let binding
[
  "let"
] @keyword

(let_bindings name: (symbol) @variable)


;; Apply
(apply_stmt . (symbol) @function)

;; Use module
[ "use" ] @keyword

(use_module (quote) . (symbol) @namespace)

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

(lambda_stmt parameters: (parameters (symbol) @variable.parameter))

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
(dot) @operator
(tilde) @operator
(backquote) @operator
(quote) @operator
(unquote) @operator
(unquote_splice) @operator

;; Highlight nil and t as constants, unlike other symbols
[
  "nil"
] @constant.builtin.boolean

;; Highlight variable names used in anamorphic macros.
[
  "it"
] @variable.builtin
