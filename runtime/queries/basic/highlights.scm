; Comments
(comment) @comment
(comment_text) @comment

; Statements (using patterns)
(print_statement) @keyword
(let_statement) @keyword
(if_statement) @keyword
(goto_statement) @keyword
(gosub_statement) @keyword
(return_statement) @keyword
(for_statement) @keyword
(next_statement) @keyword
(input_statement) @keyword
(end_statement) @keyword
(rem_statement) @keyword
(data_statement) @keyword
(read_statement) @keyword
(dim_statement) @keyword

; Built-in functions
(identifier) @function.builtin
  (#match? @function.builtin "^(ABS|SIN|COS|TAN|SQR|LEN|VAL|ASC|CHR\\$|LEFT\\$|RIGHT\\$|MID\\$)$")

; Numbers
(line_number) @number
(number) @number

; Strings
(string) @string

; Variables
(identifier) @variable

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
