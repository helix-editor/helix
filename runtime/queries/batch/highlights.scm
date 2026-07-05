(echo_off) @keyword
(comment) @comment
(label) @label

(set_keyword) @keyword
(variable_name) @variable
(set_option) @constant
(assignment_literal) @string
(arithmetic_expression) @string

; IF/FOR/GOTO/CALL statements
(if_stmt) @keyword.control.conditional
(for_stmt) @keyword.control.repeat
(goto_stmt) @keyword
(call_stmt) @keyword.function
(setlocal_stmt) @keyword
(endlocal_stmt) @keyword
(exit_stmt) @keyword

(comparison_op) @operator
(redirect_op) @operator
(fd_redirect) @operator

(command_name) @function

; CMD pseudo/dynamic environment variables
((variable_reference) @variable.builtin
 (#match? @variable.builtin "(?i)^[%!](CD|ERRORLEVEL|DATE|TIME|RANDOM|CMDCMDLINE|CMDEXTVERSION|HIGHESTNUMANODENUMBER|__APPDIR__|__CD__)[%!]$"))

(variable_reference) @variable

(for_set_literal) @string
(for_variable) @variable.parameter
(for_options) @constant

(string) @string
(integer) @constant.numeric
(command_option) @constant
(argument_value) @string
(redirect_target) @string.special

; e.g. `set INSTALL_AS_SERVICE=true`
(variable_assignment
  (assignment_value
    (assignment_literal) @constant.builtin.boolean)
  (#any-of? @constant.builtin.boolean "true" "false" "TRUE" "FALSE" "yes" "no"))

; e.g. `set PORT_MSGROUTER=10100`
(variable_assignment
  (assignment_value
    (assignment_literal) @constant.numeric)
  (#match? @constant.numeric "^\\d+$"))

