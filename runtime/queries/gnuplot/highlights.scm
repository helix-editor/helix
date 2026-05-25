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

;; Identifiers (variables)
(identifier) @variable

;; Function calls
(function_call
  (expression_list
    (expression
      (identifier) @variable.parameter)))

(function_call
  name: (_) @function)

(builtin_function) @function.builtin

;; Function definitions
(function_definition
  name: (identifier) @function)

(function_definition
  (parameter_list
    (_) @variable.parameter))

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
