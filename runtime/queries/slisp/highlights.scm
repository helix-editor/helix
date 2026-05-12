;; Keywords
[ "if" "prog" ] @keyword

;; Let binding
[ "let" ] @keyword

(let_bindings name: (symbol) @variable)

;; Apply
(apply_stmt . (symbol) @function)

;; Use module
[ "use" ] @keyword

(use_module_global (quote) . (symbol) @namespace)
(use_module_select (quote) . (symbol) @namespace)

;; Val definition
[ "val" ] @keyword

(val_definition name: (symbol) @constant)

;; External definitions
[ "ext" ] @keyword

(external_definition name: (symbol) @function)
(external_definition signature: (signature (symbol) @variable.parameter (dot) (external_type) @type.builtin))
(external_definition docstring: (string) @comment)
(external_definition return_type: (external_type) @type.builtin)

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
[ "(" ")" ] @punctuation.bracket

;; Operators
(dot) @operator
(tilde) @operator
(backquote) @operator
(quote) @operator
(unquote) @operator
(unquote_splice) @operator

;; Highlight nil t as constant
[ "nil" ] @constant.builtin

;; Highlight as t as boolean constant
[ "T" ] @constant.builtin.boolean

;; Highlight variable names used in anamorphic macros.
[ "it" ] @variable.builtin
