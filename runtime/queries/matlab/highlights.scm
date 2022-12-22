  ; highlights.scm

function_keyword: (function_keyword) @keyword.function

(function_definition
function_name: (identifier) @function
(end) @function)

(parameter_list (identifier) @variable.parameter)

[
    "if"
    "elseif"
    "else"
    "switch"
    "case"
    "otherwise"
] @keyword.control.conditional

(if_statement (end) @keyword.control.conditional)
(switch_statement (end) @keyword.control.conditional)

["for" "while"] @keyword.control.repeat
(for_statement (end) @keyword.control.repeat)
(while_statement (end) @keyword.control.repeat)

["try" "catch"] @keyword.control.exception
(try_statement (end) @keyword.control.exception)

(function_definition end: (end) @keyword)

["return" "break" "continue"] @keyword.return

(
(identifier) @constant.builtin
(#any-of? @constant.builtin "true" "false")
)

(
    (identifier) @constant.builtin
    (#eq? @constant.builtin "end")
)

;; Punctuations

[";" ","] @punctuation.special
(argument_list "," @punctuation.delimiter)
(vector_definition ["," ";"] @punctuation.delimiter)
(cell_definition ["," ";"] @punctuation.delimiter)
":" @punctuation.delimiter
(parameter_list "," @punctuation.delimiter)
(return_value "," @punctuation.delimiter)

; ;; Brackets

[
 "("
 ")"
 "["
 "]"
 "{"
 "}"
] @punctuation.bracket

;; Operators
"=" @operator
(operation [ ">"
            "<"
            "=="
            "<="
            ">="
            "=<"
            "=>"
            "~="
            "*"
            ".*"
            "/"
            "\\"
            "./"
            "^"
            ".^"
            "+"] @operator)

;; boolean operator
[
    "&&"
    "||"
] @operator

;; Number
(number) @constant.numeric

;; String
(string) @string

;; Comment
(comment) @comment
