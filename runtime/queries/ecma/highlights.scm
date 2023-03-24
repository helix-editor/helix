; Special identifiers
;--------------------

([
    (identifier)
    (shorthand_property_identifier)
    (shorthand_property_identifier_pattern)
 ] @constant
 (#match? @constant "^[A-Z_][A-Z\\d_]+$"))


((identifier) @constructor
 (#match? @constructor "^[A-Z]"))

((identifier) @variable.builtin
 (#match? @variable.builtin "^(arguments|module|console|window|document)$")
 (#is-not? local))

((identifier) @function.builtin
 (#eq? @function.builtin "require")
 (#is-not? local))

; Function and method definitions
;--------------------------------

(function
  name: (identifier) @function)
(function_declaration
  name: (identifier) @function)
(method_definition
  name: (property_identifier) @function.method)

(pair
  key: (property_identifier) @function.method
  value: [(function) (arrow_function)])

(assignment_expression
  left: (member_expression
    property: (property_identifier) @function.method)
  right: [(function) (arrow_function)])

(variable_declarator
  name: (identifier) @function
  value: [(function) (arrow_function)])

(assignment_expression
  left: (identifier) @function
  right: [(function) (arrow_function)])
  

; Function and method calls
;--------------------------

(call_expression
  function: (identifier) @function)

(call_expression
  function: (member_expression
    property: (property_identifier) @function.method))

; Variables
;----------

(identifier) @variable

; Properties
;-----------

(property_identifier) @variable.other.member
(shorthand_property_identifier) @variable.other.member
(shorthand_property_identifier_pattern) @variable.other.member

; Literals
;---------

(this) @variable.builtin
(super) @variable.builtin

[
  (true)
  (false)
  (null)
  (undefined)
] @constant.builtin

(comment) @comment

[
  (string)
  (template_string)
] @string

(regex) @string.regexp
(number) @constant.numeric.integer

; Tokens
;-------

(template_substitution
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

[
  ";"
  "?."
  "."
  ","
] @punctuation.delimiter

[
  "-"
  "--"
  "-="
  "+"
  "++"
  "+="
  "*"
  "*="
  "**"
  "**="
  "/"
  "/="
  "%"
  "%="
  "<"
  "<="
  "<<"
  "<<="
  "="
  "=="
  "==="
  "!"
  "!="
  "!=="
  "=>"
  ">"
  ">="
  ">>"
  ">>="
  ">>>"
  ">>>="
  "~"
  "^"
  "&"
  "|"
  "^="
  "&="
  "|="
  "&&"
  "||"
  "??"
  "&&="
  "||="
  "??="
  "..."
] @operator

(ternary_expression ["?" ":"] @operator)

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket

[
  "async"
  "debugger"
  "delete"
  "extends"
  "from"
  "get"
  "new"
  "set"
  "target"
  "typeof"
  "instanceof"
  "void"
  "with"
] @keyword

[
  "of"
  "as"
  "in"
] @keyword.operator

[
  "function"
] @keyword.function

[
  "class"
  "let"
  "var"
] @keyword.storage.type

[
  "const"
  "static"
] @keyword.storage.modifier

[
  "default"
  "yield"
  "finally"
  "do"
  "await"
] @keyword.control

[
  "if"
  "else"
  "switch"
  "case"
  "while"
] @keyword.control.conditional

[
  "for"
] @keyword.control.repeat

[
  "import"
  "export"
] @keyword.control.import 

[
  "return"
  "break"
  "continue"
] @keyword.control.return

[
  "throw"
  "try"
  "catch"
] @keyword.control.exception

