; hightlights.scm

[
  "."
  "@"
  "**"
  "+"
  "-"
  "~"
  "*"
  "/"
  "//"
  "/<"
  "/>"
  "div"
  "mod"
  "rem"
  ">>"
  "<<"
  "/\\"
  "^"
  "\\/"
  ".."
  "++"
  "="
  "!="
  ":="
  "=="
  "!=="
  "=:="
  "<"
  "=<"
  "<="
  ">"
  ">="
  "::"
  "in"
  "notin"
  "=.."
  "#="
  "#!="
  "#<"
  "#=<"
  "#<="
  "#>"
  "#>="
  "@<"
  "@=<"
  "@<="
  "@>"
  "@>="
  "#~"
  "#/\\"
  "#^"
  "#\\/"
  "#=>"
  "#<=>"
  "not"
  "once"
  "\\+"
  "&&"
  ";"
  "||"
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket


[
  "do"
  "else"
  "end"
  "foreach"
  "if"
  "import"
  "in"
  "index"
  "module"
  "private"
  "table"
  "then"
  "while"
  "throw"
  "true"
  "false"
  "fail"
] @keyword

(predicate_definition (predicate_rule name: (atom) @function))
(predicate_definition (predicate_fact name: (atom) @function))
(function_definition (function_rule name: (atom) @function))
(function_definition (function_fact name: (atom) @function))
(actor_definition (action_rule name: (atom) @function))
(actor_definition (nonbacktrackable_predicate_rule name: (atom) @function))

(integer) @constant.numeric.integer
(real) @constant.numeric.float
(string) @string
(comment) @comment

[
  "=>"
  "->"
  "$"
] @punctuation.special


(parameters
  [(variable) @variable.parameter
   (atom) @variable.parameter
   (array_expression [(variable) @variable.parameter (atom) @variable.parameter])
   (list_expression [(variable) @variable.parameter (atom) @variable.parameter])
   (as_pattern_expression left: [(variable) @variable.parameter (atom) @variable.parameter])])

(function_call function: (atom) @function)
(dot_expression right: (atom) @function)
