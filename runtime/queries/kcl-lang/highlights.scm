(identifier) @variable

; Reset highlighting in string interpolations
(interpolation) @none

(import_stmt
  (dotted_name
    (identifier) @namespace))

(import_stmt
  (dotted_name
    (identifier) @namespace)
  (identifier) @namespace)

(basic_type) @type

(schema_type
  (dotted_name
    (identifier) @type))

(schema_type
  (dotted_name
    (identifier) @namespace
    (identifier) @type))

(schema_expr
  (identifier) @type)

(protocol_stmt
  (identifier) @type)

(rule_stmt
    (identifier) @type)

(schema_stmt
  (identifier) @type)

(lambda_expr
  (typed_parameter (identifier) @variable.parameter))

(lambda_expr
  (identifier) @variable.parameter)

(selector_expr
  (select_suffix
    (identifier) @property))

(comment) @comment
(string) @string
(escape_sequence) @constant.character.escape

(schema_stmt
  body: (block
    .
    (string
      (string_content) @string.documentation)))

(decorator
  (identifier) @attribute)

(call_expr
  function: (identifier) @function)

(call_expr
  function: (selector_expr
    (select_suffix
      (identifier) @function)))

(integer) @constant.numeric.integer
(float) @constant.numeric.float

[
  (true)
  (false)
] @constant.builtin.boolean
[
  (none)
  (undefined)
] @constant.builtin

[
  "all"
  "any"
  "assert"
  "as"
  "check"
  "elif"
  "else"
  "filter"
  "for"
  "if"
  "import"
  "lambda"
  "map"
  "mixin"
  "protocol"
  "rule"
  "schema"
  "type"
] @keyword

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

(interpolation
  "${" @punctuation.special
  "}" @punctuation.special)

[
  "+"
  "-"
  "*"
  "**"
  "/"
  "//"
  "%"
  "<<"
  ">>"
  "&"
  "|"
  "^"
  "<"
  ">"
  "~"
  "<="
  ">="
  "=="
  "!="
  "@"
  "and"
  "or"
  "not"
  "in"
  "is"
  "="
  ":"
] @operator

; second argument is a regex in all regex functions with at least two arguments
(call_expr
  function: (selector_expr
    (identifier) @_regex)
  arguments: (argument_list
    (_)
    .
    (string
      (string_content) @string.regexp))
  (#eq? @_regex "regex"))

; first argument is a regex in 'regex.compile' function
(call_expr
  .
  function: (selector_expr
    (identifier) @_regex
    (select_suffix
      (identifier) @_fn (#eq? @_fn "compile")))
  arguments: (argument_list
    (string
      (string_content) @string.regexp))
  (#eq? @_regex "regex"))
