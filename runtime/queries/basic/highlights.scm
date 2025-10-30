; Keywords
[
  "PRINT"
  "LET"
  "IF"
  "THEN"
  "GOTO"
  "GOSUB"
  "RETURN"
  "FOR"
  "TO"
  "STEP"
  "NEXT"
  "INPUT"
  "END"
  "REM"
  "DATA"
  "READ"
  "DIM"
] @keyword

; Logical operators
[
  "AND"
  "and"
  "OR"
  "or"
  "NOT"
] @keyword.operator

; Comments
(comment) @comment
(rem_statement) @comment

; Function calls
(function_call) @function.call

; Numbers
(line_number) @constant.numeric
(number) @constant.numeric

; Strings
(string) @string

; Operators
[
  "="
  "<>"
  "<"
  ">"
  "<="
  ">="
  "+"
  "-"
  "*"
  "/"
  "^"
] @operator

; Punctuation
[
  "("
  ")"
] @punctuation.bracket

[
  ","
  ";"
] @punctuation.delimiter

; Variables
(identifier) @variable
