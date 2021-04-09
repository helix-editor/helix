(php_tag) @tag
"?>" @tag

; Types

(primitive_type) @type.builtin
(cast_type) @type.builtin
(type_name (name) @type)

; Functions

(array_creation_expression "array" @function.builtin)
(list_literal "list" @function.builtin)

(method_declaration
  name: (name) @function.method)

(function_call_expression
  function: (qualified_name (name)) @function)

(scoped_call_expression
  name: (name) @function)

(member_call_expression
  name: (name) @function.method)

(function_definition
  name: (name) @function)

; Member

(property_element
  (variable_name) @property)

(member_access_expression
  name: (variable_name (name)) @property)
(member_access_expression
  name: (name) @property)

; Variables

(relative_scope) @variable.builtin

((name) @constant
 (#match? @constant "^_?[A-Z][A-Z\d_]+$"))

((name) @constructor
 (#match? @constructor "^[A-Z]"))

((name) @variable.builtin
 (#eq? @variable.builtin "this"))

(variable_name) @variable

; Basic tokens

(string) @string
(heredoc) @string
(boolean) @constant.builtin
(null) @constant.builtin
(integer) @number
(float) @number
(comment) @comment

"$" @operator

; Keywords

"abstract" @keyword
"as" @keyword
"break" @keyword
"case" @keyword
"catch" @keyword
"class" @keyword
"const" @keyword
"continue" @keyword
"declare" @keyword
"default" @keyword
"do" @keyword
"echo" @keyword
"else" @keyword
"elseif" @keyword
"enddeclare" @keyword
"endforeach" @keyword
"endif" @keyword
"endswitch" @keyword
"endwhile" @keyword
"extends" @keyword
"final" @keyword
"finally" @keyword
"foreach" @keyword
"function" @keyword
"global" @keyword
"if" @keyword
"implements" @keyword
"include_once" @keyword
"include" @keyword
"insteadof" @keyword
"interface" @keyword
"namespace" @keyword
"new" @keyword
"private" @keyword
"protected" @keyword
"public" @keyword
"require_once" @keyword
"require" @keyword
"return" @keyword
"static" @keyword
"switch" @keyword
"throw" @keyword
"trait" @keyword
"try" @keyword
"use" @keyword
"while" @keyword
