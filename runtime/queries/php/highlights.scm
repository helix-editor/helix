(php_tag) @tag
"?>" @tag

; Types

(primitive_type) @type.builtin
(cast_type) @type.builtin
(named_type (name) @type) @type
(named_type (qualified_name) @type) @type

(namespace_definition
  name: (namespace_name (name) @namespace))

; Superglobals
(subscript_expression
  (variable_name(name) @constant.builtin
    (#match? @constant.builtin "^_?[A-Z][A-Z\\d_]+$")))

; Functions

(array_creation_expression "array" @function.builtin)
(list_literal "list" @function.builtin)

(method_declaration
  name: (name) @function.method)

(function_call_expression
  function: (_) @function)

(scoped_call_expression
  name: (name) @function)

(member_call_expression
  name: (name) @function.method)

(function_definition
  name: (name) @function)


; Member

(property_element
  (variable_name) @variable.other.member)

(member_access_expression
  name: (variable_name (name)) @variable.other.member)
(member_access_expression
  name: (name) @variable.other.member)

; Variables

(relative_scope) @variable.builtin

((name) @constant
 (#match? @constant "^_?[A-Z][A-Z\\d_]+$"))

((name) @constructor
 (#match? @constructor "^[A-Z]"))

((name) @variable.builtin
 (#eq? @variable.builtin "this"))

(variable_name) @variable

; Basic tokens

(string) @string
(heredoc) @string
(boolean) @constant.builtin.boolean
(null) @constant.builtin
(integer) @constant.numeric.integer
(float) @constant.numeric.float
(comment) @comment

"$" @operator

; Keywords

[
  "abstract" 
  "as" 
  "break" 
  "case" 
  "catch" 
  "class" 
  "const" 
  "continue" 
  "declare" 
  "default" 
  "do" 
  "echo" 
  "else" 
  "elseif" 
  "enddeclare" 
  "endforeach" 
  "endif" 
  "endswitch" 
  "endwhile" 
  "enum" 
  "extends" 
  "final" 
  "finally" 
  "foreach" 
  "fn" 
  "function" 
  "global" 
  "if" 
  "implements" 
  "include_once" 
  "include" 
  "insteadof" 
  "interface" 
  "match" 
  "namespace" 
  "new" 
  "private" 
  "protected" 
  "public" 
  "require_once" 
  "require" 
  "return" 
  "static" 
  "switch" 
  "throw" 
  "trait" 
  "try" 
  "use" 
  "while" 
] @keyword
