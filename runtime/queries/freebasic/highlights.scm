; Keywords
[
  "DIM"
  "IF"
  "THEN"
  "ELSE"
  "END IF"
  "WHILE"
  "WEND"
  "FOR"
  "TO"
  "STEP"
  "NEXT"
  "DO"
  "LOOP"
  "UNTIL"
  "SUB"
  "END SUB"
  "FUNCTION"
  "END FUNCTION"
  "RETURN"
  "PRINT"
  "INPUT"
  "SLEEP"
  "AS"
] @keyword

; Logical operators
[
  "AND"
  "OR"
  "NOT"
  "MOD"
] @keyword.operator

; Types
[
  "INTEGER"
  "LONG"
  "SINGLE"
  "DOUBLE"
  "STRING"
  "BYTE"
] @type

(type_identifier) @type

; Function and sub declarations
(sub_declaration
  name: (identifier) @function)
(function_declaration
  name: (identifier) @function)

; Function calls
(call_expression
  function: (identifier) @function.call)

; Built-in functions
((identifier) @function.builtin
  (#match? @function.builtin "^(?i)(ABS|SIN|COS|TAN|SQR|LEN|VAL|ASC|CHR|LEFT|RIGHT|MID|STR|INT|RND|INSTR|UCASE|LCASE|LTRIM|RTRIM|SPACE|TIME|DATE|TIMER)$"))

; Literals
(number_literal) @constant.numeric
(string_literal) @string

; Comments
(comment) @comment

; Operators
[
  "="
  "+"
  "-"
  "*"
  "/"
  "^"
  "<>"
  "<"
  ">"
  "<="
  ">="
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
