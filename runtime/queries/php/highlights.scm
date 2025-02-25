(php_tag) @tag
"?>" @tag

; Variables

(relative_scope) @variable.builtin

(variable_name) @variable

((name) @constant
 (#match? @constant "^_?[A-Z][A-Z\\d_]+$"))

((name) @constructor
 (#match? @constructor "^[A-Z]"))

((name) @variable.builtin
 (#eq? @variable.builtin "this"))

; Types
[
  (primitive_type)
  (cast_type)
] @type.builtin

(named_type
  [ (name) @type
    (qualified_name (name) @type)])

(base_clause
  [ (name) @type
    (qualified_name (name) @type)])

(enum_declaration
  name: (name) @type.enum)

(interface_declaration
  name: (name) @constructor)

(class_declaration
  name: (name) @constructor)

(trait_declaration
  name:(name) @constructor)

(namespace_definition
  name: (namespace_name (name) @namespace))

(namespace_name_as_prefix 
  (namespace_name (name) @namespace))

(namespace_use_clause
  [ (name) @namespace
    (qualified_name (name) @type) ])

(namespace_aliasing_clause (name) @namespace)

(class_interface_clause
  [(name) @type
   (qualified_name (name) @type)])

(scoped_call_expression
  scope: [(name) @type
          (qualified_name (name) @type)])

(class_constant_access_expression
  . [(name) @constructor
     (qualified_name (name) @constructor)]
  (name) @constant)

(use_declaration (name) @type)

(binary_expression
  operator: "instanceof"
  right: [(name) @type
          (qualified_name (name) @type)])

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

(nullsafe_member_call_expression
    name: (name) @function.method)

(object_creation_expression
  [(name) @constructor
   (qualified_name (name) @constructor)])

; Parameters
[
  (simple_parameter)
  (variadic_parameter)
] @variable.parameter

(argument
    (name) @variable.parameter)

; Member

(property_element
  (variable_name) @variable.other.member)

(member_access_expression
  name: (variable_name (name)) @variable.other.member)
(member_access_expression
  name: (name) @variable.other.member)

; Attributes
(attribute_list) @attribute

; Basic tokens

[
  (string)
  (encapsed_string)
  (heredoc_body)
  (nowdoc_body)
  (shell_command_expression) 
] @string
(escape_sequence) @constant.character.escape

(boolean) @constant.builtin.boolean
(null) @constant.builtin
(integer) @constant.numeric.integer
(float) @constant.numeric.float
(comment) @comment

(goto_statement (name) @label)
(named_label_statement (name) @label)

; Keywords

[
  "default" 
  "echo" 
  "enum" 
  "extends" 
  "final" 
  "goto"
  "global" 
  "implements" 
  "insteadof" 
  "new" 
  "private" 
  "protected" 
  "public" 
  "clone"
  "unset"
] @keyword

[
  "if" 
  "else" 
  "elseif" 
  "endif" 
  "switch" 
  "endswitch" 
  "case" 
  "match" 
  "declare" 
  "enddeclare" 
  "??"
] @keyword.control.conditional

[
  "for"
  "endfor"
  "foreach" 
  "endforeach" 
  "while" 
  "endwhile" 
  "do"
] @keyword.control.repeat

[
  
  "include_once" 
  "include" 
  "require_once" 
  "require" 
  "use"
] @keyword.control.import

[
  "return" 
  "break" 
  "continue" 
  "yield"
] @keyword.control.return

[
  "throw" 
  "try" 
  "catch" 
  "finally"
] @keyword.control.exception

[
  "as" 
  "or"
  "xor"
  "and"
  "instanceof"
] @keyword.operator

[
  "fn" 
  "function" 
] @keyword.function

[
  "namespace" 
  "class" 
  "interface" 
  "trait" 
  "abstract" 
] @keyword.storage.type

[
  "static"
  "const"
] @keyword.storage.modifier

[
  ","
  ";"
  ":"
  "\\"
 ] @punctuation.delimiter

[
  (php_tag)
  "?>"
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "#["
] @punctuation.bracket

[
  "="

  "."
  "-"
  "*"
  "/"
  "+"
  "%"
  "**"

  "~"
  "|"
  "^"
  "&"
  "<<"
  ">>"

  "->"
  "?->"

  "=>"

  "<"
  "<="
  ">="
  ">"
  "<>"
  "=="
  "!="
  "==="
  "!=="

  "!"
  "&&"
  "||"

  ".="
  "-="
  "+="
  "*="
  "/="
  "%="
  "**="
  "&="
  "|="
  "^="
  "<<="
  ">>="
  "??="
  "--"
  "++"

  "@"
  "::"
] @operator
