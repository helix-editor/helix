[
  "="
  "+"
  "-"
  "*"
  "/"
  "%"
  "^"
  "+="
  "-="
  "*="
  "/="
  "%="
  "^="
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  ".."
  "..="
  "->"
  (null_check)
] @operator

[
  "let"
] @keyword

[
  "and"
  "not"
  "or"
] @keyword.operator

[
  "return"
  "yield"
] @keyword.control.return

[
  "if"
  "then"
  "else"
  "else if"
  "match"
  "switch"
] @keyword.control.conditional

[
  (break)
  (continue)
  "for"
  "in"
  "loop"
  "until"
  "while"
] @keyword.control.repeat

[
  "throw"
  "try"
  "catch"
  "finally"
] @keyword.control.exception

[
  "export"
  "from"
  "import"
  "as"
] @keyword.control.import

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "|"
] @punctuation.bracket

(string (interpolation ["{" "}"] @punctuation.special))

[
  ";"
  ":"
  ","
] @punctuation.delimiter

(identifier) @variable

(import_module
  (identifier) @namespace)

(import_item
  (identifier) @namespace)

(export
  (identifier) @namespace)

(chain
  start: (identifier) @function)

(chain
  (lookup (identifier)) @variable.other.member)

(call
  function: (identifier)) @function

(call_arg
  (identifier) @variable.other.member)

[
  (true)
  (false)
] @constant.builtin.boolean

(comment) @comment

(debug) @keyword

(string) @string

(fill_char) @punctuation.delimiter

(alignment) @operator

(escape) @constant.character.escape

(null) @constant.builtin

(number) @constant.numeric

(meta) @keyword.directive

(meta
  name: (identifier) @variable.other.member)

(entry_inline
  key: (identifier) @variable.other.member)

(entry_block
  key: (identifier) @variable.other.member)

(self) @variable.builtin

(type
  _ @type)

(arg
  (_ (identifier) @variable.parameter))

(ellipsis) @variable.parameter
