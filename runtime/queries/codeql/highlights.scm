[
  "and"
  "any"
  "as"
  "asc"
  "avg"
  "by"
  "class"
  "concat"
  "count"
  "desc"
  "else"
  "exists"
  "extends"
  "forall"
  "forex"
  "from"
  "if"
  "implements"
  "implies"
  "import"
  "in"
  "instanceof"
  "max"
  "min"
  "module"
  "newtype"
  "not"
  "or"
  "order"
  "rank"
  "select"
  "strictconcat"
  "strictcount"
  "strictsum"
  "sum"
  "then"
  "where"

  (false)
  (predicate)
  (result)
  (specialId)
  (super)
  (this)
  (true)
] @keyword

[
  "boolean"
  "float"
  "int"
  "date"
  "string"
] @type.builtin

(annotName) @attribute

[
  "<"
  "<="
  "="
  ">"
  ">="
  "-"
  "!="
  "/"
  "*"
  "%"
  "+"
  "::"
] @operator

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

[
  ","
  "|"
] @punctuation.delimiter

(className) @type

(varName) @variable

(integer) @constant.numeric.integer
(float) @constant.numeric.float

(string) @string

(aritylessPredicateExpr (literalId) @function)
(predicateName) @function

[
  (line_comment)
  (block_comment)
  (qldoc)
] @comment
