;; Operators

[
 "&&"
 "||"
 "|"
 "&"
 "="
 "!="
 ".."
 "!"
 (direction)
 (stream_redirect)
 (test_option)
] @operator

[
 "not"
 "and"
 "or"
] @keyword.operator

;; Conditionals

(if_statement
[
 "if"
 "end"
] @keyword.control.conditional)

(switch_statement
[
 "switch"
 "end"
] @keyword.control.conditional)

(case_clause
[
 "case"
] @keyword.control.conditional)

(else_clause 
[
 "else"
] @keyword.control.conditional)

(else_if_clause 
[
 "else"
 "if"
] @keyword.control.conditional)

;; Loops/Blocks

(while_statement
[
 "while"
 "end"
] @keyword.control.repeat)

(for_statement
[
 "for"
 "end"
] @keyword.control.repeat)

(begin_statement
[
 "begin"
 "end"
] @keyword.control.repeat)

;; Keywords

[
 "in"
 (break)
 (continue)
] @keyword

"return" @keyword.control.return

;; Punctuation

[
 "["
 "]"
 "{"
 "}"
 "("
 ")"
] @punctuation.bracket

"," @punctuation.delimiter

;; Commands

(command
  argument: [
             (word) @variable.parameter (#match? @variable.parameter "^-")
            ]
)

; derived from builtin -n (fish 3.7.1)
(command
  name: [
    (word) @function.builtin
    (#any-of? @function.builtin "abbr" "alias" "and" "argparse" "begin" "bg" "bind" "block" "break" "breakpoint" "builtin" "case" "cd" "command" "commandline" "complete" "contains" "continue" "count" "disown" "echo" "else" "emit" "end" "eval" "exec" "exit" "false" "fg" "for" "function" "functions" "history" "if" "isatty" "jobs" "math" "not" "or" "path" "printf" "pwd" "random" "read" "realpath" "return" "set" "set_color" "source" "status" "string" "switch" "test" "time" "true" "type" "ulimit" "wait" "while")
  ]
)

(test_command "test" @function.builtin)

; non-builtin command names
(command name: (word) @function)

;; Functions

(function_definition ["function" "end"] @keyword.function)

(function_definition
  name: [
        (word) (concatenation)
        ] 
@function)

(function_definition
  option: [
          (word)
          (concatenation (word))
          ] @variable.parameter (#match? @variable.parameter "^-")
)

;; Strings

[(double_quote_string) (single_quote_string)] @string
(escape_sequence) @constant.character.escape

;; Variables

(variable_name) @variable
(variable_expansion) @constant

;; Nodes

(integer) @constant.numeric.integer
(float) @constant.numeric.float
(comment) @comment
(test_option) @string

((word) @constant.builtin.boolean
(#match? @constant.builtin.boolean "^(true|false)$"))

;; Error

(ERROR) @error
