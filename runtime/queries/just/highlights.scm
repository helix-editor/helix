(assignment (NAME) @variable)
(alias (NAME) @variable)
(value (NAME) @variable)
(parameter (NAME) @variable)
(setting (NAME) @keyword)
(setting "shell" @keyword)

(call (NAME) @function)
(dependency (NAME) @function)
(depcall (NAME) @function)
(recipeheader (NAME) @function)

(depcall (expression) @variable.parameter)
(parameter) @variable.parameter
(variadic_parameters) @variable.parameter

["if" "else"] @keyword.control.conditional

(string) @string

(boolean ["true" "false"]) @constant.builtin.boolean

(comment) @comment

; (interpolation) @string

(shebang interpreter:(TEXT) @keyword ) @comment

["export" "alias" "set"] @keyword

["@" "==" "!=" "+" ":="] @operator

[ "(" ")" "[" "]" "{{" "}}" "{" "}"] @punctuation.bracket
