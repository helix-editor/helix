[
  (comment)
  (multiline_comment)
] @comment

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "<"
  ">"
] @punctuation.bracket

[
  ","
  ";"
  "."
] @punctuation.delimiter

[
  "#"
  "-"
  "..."
  ":"
] @operator

[
  "="
  "!cond"
  (operator_keyword)
] @function

[
  "true"
  "false"
] @constant.builtin.boolean

[
  "?"
] @constant.builtin

(var) @variable

(template_arg (identifier) @variable.parameter)

(_ argument: (value (identifier) @variable.parameter))

(type) @type

"code" @type.builtin

(number) @constant.numeric.integer
[
  (string_string)
  (code_string)
] @string

(preprocessor) @keyword.directive

[
  "class"
  "field"
  "let"
  "defvar"
  "def"
  "defset"
  "defvar"
  "deftype"
  "assert"
  "dump"
] @keyword

[
  "let"
  "in"
  "foreach"
  "if"
  "then"
  "else"
] @keyword.operator

"include" @keyword.control.import

[
  "multiclass"
  "defm"
] @namespace

(ERROR) @error
