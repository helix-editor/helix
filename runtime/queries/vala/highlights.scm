; highlights.scm

; highlight constants
(
  (member_access_expression (identifier) @constant)
  (#match? @constant "^[A-Z][A-Z_0-9]*$")
)

(
  (member_access_expression (member_access_expression) @include (identifier) @constant)
  (#match? @constant "^[A-Z][A-Z_0-9]*$")
)

(comment) @comment

(type (symbol (_)? @include (identifier) @type))

; highlight creation methods in object creation expressions
(
  (object_creation_expression (type (symbol (symbol (symbol)? @include (identifier) @type) (identifier) @constructor)))
  (#match? @constructor "^[a-z][a-z_0-9]*$")
)

(unqualified_type (symbol . (identifier) @type))
(unqualified_type (symbol (symbol) @include (identifier) @type))

(attribute) @attribute
(method_declaration (symbol (symbol) @type (identifier) @function))
(method_declaration (symbol (identifier) @function))
(local_function_declaration (identifier) @function)
(destructor_declaration (identifier) @function)
(creation_method_declaration (symbol (symbol) @type (identifier) @constructor))
(creation_method_declaration (symbol (identifier) @constructor))
(enum_declaration (symbol) @type)
(enum_value (identifier) @constant)
(errordomain_declaration (symbol) @type)
(errorcode (identifier) @constant)
(constant_declaration (identifier) @constant)
(method_call_expression (member_access_expression (identifier) @function))
(lambda_expression (identifier) @parameter)
(parameter (identifier) @parameter)
(property_declaration (symbol (identifier) @property))
(field_declaration (identifier) @field)
[
 (this_access)
 (base_access)
 (value_access)
] @variable.builtin
(boolean) @boolean
(character) @character
(integer) @number
(null) @constant.builtin
(real) @float
(regex) @constant
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

"using" @include

(symbol "global::" @include)

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
] @repeat

[
  "catch"
  "finally"
  "throw"
  "throws"
  "try"
] @exception

[
  "return"
  "yield"
] @keyword.return

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
