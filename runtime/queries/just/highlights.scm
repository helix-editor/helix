(assignment (NAME) @variable)
(alias (NAME) @variable)
(value (NAME) @variable)
(parameter (NAME) @variable)
(setting (NAME) @keyword)
(setting "shell" @keyword)

(call (NAME) @keyword.function)
(dependency (NAME) @keyword.function)
(depcall (NAME) @keyword.function)
(recipeheader (NAME) @keyword.function)

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

["@" "==" "!=" "+" ":="] @keyword.operator

[ "(" ")" "[" "]" "{{" "}}" "{" "}"] @punctuation.bracket

(ERROR) @error
