; highlights.scm

; highlight constants
(
  (member_access_expression (identifier) @constant)
  (#match? @constant "^[A-Z][A-Z_0-9]*$")
)

(
  (member_access_expression (member_access_expression) @namespace (identifier) @constant)
  (#match? @constant "^[A-Z][A-Z_0-9]*$")
)

(comment) @comment

(type (symbol (_)? @namespace (identifier) @type))

; highlight creation methods in object creation expressions
(
  (object_creation_expression (type (symbol (symbol (symbol)? @namespace (identifier) @type) (identifier) @constructor)))
  (#match? @constructor "^[a-z][a-z_0-9]*$")
)

(unqualified_type (symbol . (identifier) @type))
(unqualified_type (symbol (symbol) @namespace (identifier) @type))

(attribute) @variable.other.member
(method_declaration (symbol (symbol) @type (identifier) @function))
(method_declaration (symbol (identifier) @function))
(local_function_declaration (identifier) @function)
(destructor_declaration (identifier) @function)
(creation_method_declaration (symbol (symbol (identifier) @type) (identifier) @constructor))
(creation_method_declaration (symbol (identifier) @constructor))
(enum_declaration (symbol) @type)
(enum_value (identifier) @constant)
(errordomain_declaration (symbol) @type)
(errorcode (identifier) @constant)
(constant_declaration (identifier) @constant)
(method_call_expression (member_access_expression (identifier) @function))
(lambda_expression (identifier) @variable.parameter)
(parameter (identifier) @variable.parameter)
(property_declaration (symbol (identifier) @variable.other.member))
(field_declaration (identifier) @variable)
(identifier) @variable
[
 (this_access)
 (base_access)
 (value_access)
] @variable.builtin
(boolean) @constant.builtin.boolean
(character) @constant.character
(integer) @constant.numeric.integer
(null) @constant.builtin
(real) @constant.numeric.float
(regex) @string.regexp
(string) @string
[
 (escape_sequence)
 (string_formatter)
] @string.special
(template_string) @string
(template_string_expression) @string.special
(verbatim_string) @string
[
 "var"
 "void"
] @type.builtin

[
 "abstract"
 "async"
 "break"
 "case"
 "catch"
 "class"
 "const"
 "construct"
 "continue"
 "default"
 "delegate"
 "do"
 "dynamic"
 "else"
 "enum"
 "errordomain"
 "extern"
 "finally"
 "for"
 "foreach"
 "get"
 "if"
 "inline"
 "interface"
 "internal"
 "lock"
 "namespace"
 "new"
 "out"
 "override"
 "owned"
 "partial"
 "private"
 "protected"
 "public"
 "ref"
 "set"
 "signal"
 "static"
 "struct"
 "switch"
 "throw"
 "throws"
 "try"
 "unowned"
 "virtual"
 "weak"
 "while"
 "with"
] @keyword

[
  "and"
  "as"
  "delete"
  "in"
  "is"
  "not"
  "or"
  "sizeof"
  "typeof"
] @keyword.operator

"using" @namespace

(symbol "global::" @namespace)

(array_creation_expression "new" @keyword.operator)
(object_creation_expression "new" @keyword.operator)
(argument "out" @keyword.operator)
(argument "ref" @keyword.operator)

[
  "continue"
  "do"
  "for"
  "foreach"
  "while"
] @keyword.control.repeat

[
  "catch"
  "finally"
  "throw"
  "throws"
  "try"
] @keyword.control.exception

[
  "return"
  "yield"
] @keyword.control.return

[
 "="
 "=="
 "+"
 "+="
 "-"
 "-="
 "++"
 "--"
 "|"
 "|="
 "&"
 "&="
 "^"
 "^="
 "/"
 "/="
 "*"
 "*="
 "%"
 "%="
 "<<"
 "<<="
 ">>"
 ">>="
 "."
 "?."
 "->"
 "!"
 "!="
 "~"
 "??"
 "?"
 ":"
 "<"
 "<="
 ">"
 ">="
 "||"
 "&&"
 "=>"
] @operator

[
 ","
 ";"
] @punctuation.delimiter

[
 "("
 ")"
 "{"
 "}"
 "["
 "]"
] @punctuation.bracket
