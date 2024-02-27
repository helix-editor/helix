[
  "!instanceof"
  "assert"
  "class"
  "extends"
  "instanceof"
  "package"
] @keyword

[
  "!in"
  "as"
  "in"
] @keyword.operator

[
  "case"
  "default"
  "else"
  "if"
  "switch"
] @keyword.control.conditional

[
  "catch"
  "finally"
  "try"
] @keyword.control.exception

"def" @keyword.function

"import" @keyword.control.import

[
  "for"
  "while"
  (break)
  (continue)
] @keyword.control.repeat

"return" @keyword.control.return

[
  "true"
  "false"
] @constant.builtin.boolean

(null) @constant.builtin

"this" @variable.builtin

[
  "int"
  "char"
  "short"
  "long"
  "boolean"
  "float"
  "double"
  "void"
] @type.builtin

[
  "final"
  "private"
  "protected"
  "public"
  "static"
  "synchronized"
] @keyword.storage.modifier

(comment) @comment

(shebang) @keyword.directive

(string) @string

(string
  (escape_sequence) @constant.character.escape)

(string
  (interpolation
    "$" @punctuation.special))

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  ":"
  ","
  "."
] @punctuation.delimiter

(number_literal) @constant.numeric

(identifier) @variable

((identifier) @constant
  (#match? @constant "^[A-Z][A-Z_]+"))

[
  "%"
  "*"
  "/"
  "+"
  "-"
  "<<"
  ">>"
  ">>>"
  ".."
  "..<"
  "<..<"
  "<.."
  "<"
  "<="
  ">"
  ">="
  "=="
  "!="
  "<=>"
  "==="
  "!=="
  "=~"
  "==~"
  "&"
  "^"
  "|"
  "&&"
  "||"
  "?:"
  "+"
  "*"
  ".&"
  ".@"
  "?."
  "*."
  "*"
  "*:"
  "++"
  "--"
  "!"
] @operator

(string
  "/" @string)

(ternary_op
  ([
    "?"
    ":"
  ]) @keyword.operator)

(map
  (map_item
    key: (identifier) @variable.parameter))

(parameter
  type: (identifier) @type
  name: (identifier) @variable.parameter)

(generic_param
  name: (identifier) @variable.parameter)

(declaration
  type: (identifier) @type)

(function_definition
  type: (identifier) @type)

(function_declaration
  type: (identifier) @type)

(class_definition
  name: (identifier) @type)

(class_definition
  superclass: (identifier) @type)

(generic_param
  superclass: (identifier) @type)

(type_with_generics
  (identifier) @type)

(type_with_generics
  (generics
    (identifier) @type))

(generics
  [
    "<"
    ">"
  ] @punctuation.bracket)

(generic_parameters
  [
    "<"
    ">"
  ] @punctuation.bracket)

; TODO: Class literals with PascalCase
(declaration
  "=" @operator)

(assignment
  "=" @operator)

(function_call
  function: (identifier) @function)

(function_call
  function:
    (dotted_identifier
      (identifier) @function .))

(function_call
  (argument_list
    (map_item
      key: (identifier) @variable.parameter)))

(juxt_function_call
  function: (identifier) @function)

(juxt_function_call
  function:
    (dotted_identifier
      (identifier) @function .))

(juxt_function_call
  (argument_list
    (map_item
      key: (identifier) @variable.parameter)))

(function_definition
  function: (identifier) @function)

(function_declaration
  function: (identifier) @function)

(annotation) @function.macro

(annotation
  (identifier) @function.macro)

"@interface" @function.macro

(groovy_doc) @comment.block.documentation

(groovy_doc
  [
    (groovy_doc_param)
    (groovy_doc_throws)
    (groovy_doc_tag)
  ] @string.special)

(groovy_doc
  (groovy_doc_param
    (identifier) @variable.parameter))

(groovy_doc
  (groovy_doc_throws
    (identifier) @type))
