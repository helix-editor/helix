[
 "while"
 (break)
 (continue)
] @keyword.repeat

[
 "if"
 "else"
] @keyword.conditional

"fn" @keyword.function

"return" @keyword.return

[
  "+"
  "-"
  "/"
  "*"
  "**"
  "%"
  "<"
  ">"
  "=="
  "="
  "+="
  "-="
  "/="
  "*="
  "**"
  "%="
] @operator


[
  ":"
  ","
  "."
] @punctuation.delimiter

[
 "("
 ")"
 "["
 "]"
 "{"
 "}"
] @punctuation.bracket

(parameters (identifier) @variable.parameter)

(call (identifier) @function.call)

(identifier) @variable

[(none) (true) (false)]  @constant.builtin

[(true) (false)] @boolean

(int) @number
(float) @number.float

(string) @string

(comment) @comment
