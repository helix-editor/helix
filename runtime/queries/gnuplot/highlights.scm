;; Keywords (commands)
;; Core commands
"plot" @keyword.control
"splot" @keyword.control
"fit" @keyword.control
"set" @keyword.control
"unset" @keyword.control
"load" @keyword.control
"pause" @keyword.control
(reset_command) @keyword.control
"do" @keyword.control.repeat
"for" @keyword.control.repeat

;; array definition
"array" @type

;; Plot / fit modifiers
"using" @keyword.operator
"with" @keyword.operator
"title" @keyword.operator
"notitle" @keyword.operator
"via" @keyword.operator

;; Function calls
(function_call
  name: (identifier) @function
)
(function_call
  (identifier) @variable.parameter
)
(builtin_function) @function.builtin

;; Function definitions
(function_definition
  name: (identifier) @function
)

;; Identifiers (variables)
(identifier) @variable

;; Numbers (distinct integer/float if desired):
(number) @constant.numeric.float

;; Strings
(string) @string

;; Comments
(comment) @comment

;; Operators
(operator) @keyword.operator

;; Range literal
(range) @constant

;; Punctuation
["(" ")" "[" "]" "," ":" "="] @punctuation.bracket
