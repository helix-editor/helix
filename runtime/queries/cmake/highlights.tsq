[
  (quoted_argument)
  (bracket_argument)
 ] @string

(variable) @variable

[
  (bracket_comment)
  (line_comment)
 ] @comment

(normal_command (identifier) @function)

["ENV" "CACHE"] @string.special.symbol
["$" "{" "}" "<" ">"] @punctuation
["(" ")"] @punctuation.bracket

[
  (function)
  (endfunction)
  (macro)
  (endmacro)
 ] @keyword.function

[
  (if)
  (elseif)
  (else)
  (endif)
 ] @keyword.control.conditional

[
  (foreach)
  (endforeach)
  (while)
  (endwhile)
 ] @keyword.control.repeat

(function_command
   (function)
   . (argument) @function
   (argument)* @variable.parameter
 )

(macro_command
   (macro)
   . (argument) @function.macro
   (argument)* @variable.parameter
 )

(normal_command
  (identifier) @function.builtin
  . (argument) @variable
  (#match? @function.builtin "^(?i)(set)$"))

(normal_command
  (identifier) @function.builtin
  . (argument)
  (argument) @constant
  (#match? @constant "^(?:PARENT_SCOPE|CACHE)$")
  (#match? @function.builtin "^(?i)(unset)$"))

(normal_command
  (identifier) @function.builtin
  . (argument)
  . (argument)
  (argument) @constant
  (#match? @constant "^(?:PARENT_SCOPE|CACHE|FORCE)$")
  (#match? @function.builtin "^(?i)(set)$")
 )

((argument) @constant.builtin.boolean
   (#match? @constant.builtin.boolean "^(?i)(?:1|on|yes|true|y|0|off|no|false|n|ignore|notfound|.*-notfound)$")
 )

(if_command
   (if)
   (argument) @operator
   (#match? @operator "^(?:NOT|AND|OR|COMMAND|POLICY|TARGET|TEST|DEFINED|IN_LIST|EXISTS|IS_NEWER_THAN|IS_DIRECTORY|IS_SYMLINK|IS_ABSOLUTE|MATCHES|LESS|GREATER|EQUAL|LESS_EQUAL|GREATER_EQUAL|STRLESS|STRGREATER|STREQUAL|STRLESS_EQUAL|STRGREATER_EQUAL|VERSION_LESS|VERSION_GREATER|VERSION_EQUAL|VERSION_LESS_EQUAL|VERSION_GREATER_EQUAL)$")
)

(normal_command
   (identifier) @function.builtin
   . (argument)
   (argument) @constant
   (#match? @constant "^(?:ALL|COMMAND|DEPENDS|BYPRODUCTS|WORKING_DIRECTORY|COMMENT|JOB_POOL|VERBATIM|USES_TERMINAL|COMMAND_EXPAND_LISTS|SOURCES)$")
   (#match? @function.builtin "^(?i)(add_custom_target)$")
 )

(normal_command
   (identifier) @function.builtin
   (argument) @constant
   (#match? @constant "^(?:OUTPUT|COMMAND|MAIN_DEPENDENCY|DEPENDS|BYPRODUCTS|IMPLICIT_DEPENDS|WORKING_DIRECTORY|COMMENT|DEPFILE|JOB_POOL|VERBATIM|APPEND|USES_TERMINAL|COMMAND_EXPAND_LISTS)$")
   (#match? @function.builtin "^(?i)(add_custom_command)$")
 )

