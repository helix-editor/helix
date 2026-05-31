(comment) @comment

(number) @constant.numeric.integer
(bool) @constant.builtin.boolean
(string) @string
(escape_sequence) @constant.character.escape

[
  "(" ")" "[" "]" "{" "}"
] @punctuation.bracket

[
  "," "." ":"
] @punctuation.delimiter

[
  "=" "==" "!=" "+" "+=" "-=" "/" "/=" "<" ">" ">=" "?"
] @operator

[
  "and" "or" "not" "in"
] @keyword.operator

[
  "if" "elif" "else" "endif"
] @keyword.control.conditional

[
  "foreach" "endforeach"
  (keyword_break)
  (keyword_continue)
] @keyword.control.repeat

; Format-string placeholder `@var@`.
"@" @punctuation.special

(identifier) @variable

; Command calls: `project(...)`, `executable(...)`.
(normal_command
  command: (identifier) @function)

; Method calls: `obj.method(...)` — the property is the method name.
(expression_statement
  property: (identifier) @function.method)

; Dictionary / keyword-argument keys.
(pair
  key: (identifier) @variable.other.member)
