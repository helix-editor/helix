[
  "="
  "+"
  "-"
  "*"
  "/"
  "%"
  "+="
  "-="
  "*="
  "/="
  "%="
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
] @keyword.return

[
  "if"
  "then"
  "else"
  "else if"
  "match"
  "switch"
] @keyword.conditional

[
  (break)
  (continue)
  "for"
  "in"
  "loop"
  "until"
  "while"
] @keyword.repeat

[
  "throw"
  "try"
  "catch"
  "finally"
] @keyword.exception

[
  "export"
  "from"
  "import"
  "as"
] @keyword.import

(string (interpolation ("{") @punctuation.special))
(string (interpolation ("}") @punctuation.special))

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "|"
] @punctuation.bracket

[
  ";"
  ":"
  ","
] @punctuation.delimiter

(import_module
  (identifier) @module)

(import_item
  (identifier) @module)

(export
  (identifier) @module)

(call
  function: (identifier) @function.method)

(chain
  lookup: (identifier) @variable.other.member)

[
  (true)
  (false)
] @constant.builtin.boolean

(comment) @comment

(debug) @keyword.debug

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

(variable
  type: (identifier) @type)

(arg
  (_ (identifier) @variable.parameter))

(ellipsis) @variable.parameter

(function
  output_type: (identifier) @type)

(identifier) @variable
