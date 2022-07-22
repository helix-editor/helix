;; SPDX-License-Identifier: 0BSD
;; SPDX-FileCopyrightText: 2022 Tobias Frilling

(comment) @comment

(if "if" @keyword.control.conditional)
(if (elif "elif" @keyword.control.conditional))
(if (else "else" @keyword.control.conditional))

(while "while" @keyword.control.repeat)
(while (else "else" @keyword.control.repeat))
(for "for" @keyword.control.repeat)
(for (else "else" @keyword.control.repeat))

(try "try" @keyword.control.exception)
(try (catch "catch" @keyword.control.exception))
(try (else "else" @keyword.control.exception))
(try (finally "finally" @keyword.control.exception))

(import "use" @keyword.control.import)
(import (bareword) @string.special)

(wildcard ["*" "**" "?"] @string.special)

(command argument: (bareword) @variable.parameter)
(command head: (identifier) @function)
((command head: (identifier) @keyword.control.return)
 (#eq? @keyword.control.return "return"))
((command (identifier) @keyword.operator)
 (#match? @keyword.operator "(and|or|coalesce)"))
((command head: _ @function)
 (#match? @function "([+]|[-]|[*]|[/]|[%]|[<]|[<][=]|[=][=]|[!][=]|[>]|[>][=]|[<][s]|[<][=][s]|[=][=][s]|[!][=][s]|[>][s]|[>][=][s])"))

(pipeline "|" @operator)
(redirection [">" "<" ">>" "<>"] @operator)

(io_port) @constant.numeric

(function_definition
  "fn" @keyword.function
  (identifier) @function)

(parameter_list) @variable.parameter
(parameter_list "|" @punctuation.bracket)

(variable_declaration
  "var" @keyword
  (lhs (identifier) @variable))

(variable_assignment
  "set" @keyword
  (lhs (identifier) @variable))

(temporary_assignment
  "tmp" @keyword
  (lhs (identifier) @variable))

(variable_deletion
  "del" @keyword
  (identifier) @variable)


(number) @constant.numeric
(string) @string

((variable (identifier) @function)
  (#match? @function ".+\\~$"))
((variable (identifier) @constant.builtin.boolean)
 (#match? @constant.builtin.boolean "(true|false)"))
((variable (identifier) @constant.builtin)
 (#match? @constant.builtin "(_|after-chdir|args|before-chdir|buildinfo|nil|notify-bg-job-success|num-bg-jobs|ok|paths|pid|pwd|value-out-indicator|version)"))
(variable (identifier) @variable)

["$" "@"] @punctuation.special
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
";" @punctuation.delimiter
