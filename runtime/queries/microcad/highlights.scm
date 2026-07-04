; Variables
;----------

(identifier) @variable
(type) @type

; Function and method definitions
;--------------------------------

(function_definition
  name: (identifier) @function)
(parameter (identifier) @variable.parameter)

; Function and method calls
;--------------------------

(call
  method: (qualified_name) @function)
(method_call
  method: (qualified_name) @function.method)

; Literals
;---------

[
  "true"
  "false"
] @constant.builtin

(comment) @comment
(doc_comment) @comment
(inner_doc_comment) @comment

(string) @string

(number_literal) @number

; Tokens
;-------

[
  ";"
  "."
  ","
] @punctuation.delimiter

[
  "-"
  "+"
  "*"
  "/"
  "<"
  "<="
  "="
  "=="
  "!"
  "!="
  ">"
  ">="
  "^"
  "&"
  "|"
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket

(format_expression
  "{" @punctuation.special
  "}" @punctuation.special) @embedded

[
  "mod"
  "part"
  "sketch"
  "op"
  "fn"
  "if"
  "else"
  "use"
  "as"
  "return"
  "const"
  "prop"
  "init"
  "__plugin"
  "assembly"
  "material"
  "unit"
  "enum"
  "struct"
  "match"
  "type"
] @keyword
(visibility) @keyword

