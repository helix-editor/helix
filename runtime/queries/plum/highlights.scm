; General patterns come first and more specific ones later: for the same node
; the last matching pattern wins, and an inner node wins over its parent.

; Punctuation

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  ","
  "."
  ":"
] @punctuation.delimiter

; Operators

[
  "+"
  "-"
  "*"
  "/"
  "//"
  "%"
  "**"
  "&"
  "|"
  "^"
  "~"
  "<<"
  ">>"
  "=="
  "!="
  "<"
  "<="
  ">"
  ">="
  "="
  "+="
  "-="
  "*="
  "/="
  "//="
  "%="
  "**="
  "&="
  "|="
  "^="
  "<<="
  ">>="
  "??"
  "?."
  "?"
  ".."
  "..="
  "->"
  "=>"
] @operator

; Variables

(identifier) @variable

((identifier) @type
  (#match? @type "^[A-Z]"))

((identifier) @constant
  (#match? @constant "^_*[A-Z][A-Z0-9_]*$"))

(constant_declaration
  name: (identifier) @constant)

[
  (self)
  (super)
] @variable.builtin

(wildcard) @comment.unused

; Fields

(field_declaration
  name: (identifier) @variable.other.member)

(member_expression
  field: (identifier) @variable.other.member)

; Parameters

(parameter
  name: (identifier) @variable.parameter)

(lambda_parameter
  name: (identifier) @variable.parameter)

(keyword_argument
  name: (identifier) @variable.parameter)

; Functions

(call_expression
  function: (identifier) @function)

((call_expression
  function: (identifier) @constructor)
  (#match? @constructor "^[A-Z]"))

((call_expression
  function: (identifier) @function.builtin)
  (#any-of? @function.builtin
    "drop" "ask" "len" "int" "float" "str" "byte" "type_of" "assert" "exit" "set"))

(call_expression
  function: (member_expression
    field: (identifier) @function.method))

(function_declaration
  name: (identifier) @function)

(method_declaration
  name: (identifier) @function.method)

; Types

(type_identifier) @type

((type_identifier) @type.builtin
  (#any-of? @type.builtin
    "int" "float" "bool" "str" "byte" "list" "map" "set" "Channel" "Json"))

(tree_declaration
  name: (type_identifier) @type)

(trait_declaration
  name: (type_identifier) @type)

(kind_declaration
  name: (type_identifier) @type)

(kind_case
  name: (identifier) @type.enum.variant)

(scoped_type
  module: (identifier) @namespace)

; Modules

(seed_statement
  path: (module_path
    (identifier) @namespace))

(seed_statement
  alias: (identifier) @namespace)

(seed_statement
  path: (relative_path) @string.special.path)

; Literals

(integer) @constant.numeric.integer

(float) @constant.numeric.float

(boolean) @constant.builtin.boolean

(none) @constant.builtin

(string) @string

(escape_sequence) @constant.character.escape

(format_specifier) @string.special

(interpolation
  "{" @punctuation.special
  "}" @punctuation.special)

; Keywords

[
  "and"
  "or"
  "not"
  "in"
  "is"
  "as"
] @keyword.operator

[
  "plant"
  "root"
  "tree"
  "trait"
  "kind"
] @keyword.storage.type

"weak" @keyword.storage.modifier

"branch" @keyword.function

"seed" @keyword.control.import

(seed_statement
  "as" @keyword.control.import)

[
  "if"
  "else"
  "match"
] @keyword.control.conditional

[
  "for"
  "while"
  (break_statement)
  (continue_statement)
] @keyword.control.repeat

(for_statement
  "in" @keyword.control.repeat)

"return" @keyword.control.return

[
  "try"
  "catch"
  "finally"
  "throw"
] @keyword.control.exception

"spawn" @keyword.control

; Comments

(comment) @comment.line
