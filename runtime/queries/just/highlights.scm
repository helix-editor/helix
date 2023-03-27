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

(depcall (expression) @parameter)
(parameter) @parameter
(variadic_parameters) @parameter

["if" "else"] @conditional

(string) @string

(boolean ["true" "false"]) @boolean

(comment) @comment

; (interpolation) @string

(shebang interpreter:(TEXT) @keyword ) @comment

["export" "alias" "set"] @keyword

["@" "==" "!=" "+" ":="] @operator

[ "(" ")" "[" "]" "{{" "}}" "{" "}"] @punctuation.bracket

(ERROR) @error
