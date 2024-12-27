((program
  .
  (comment)*
  .
  (comment) @keyword.import)
  (#match? @keyword.import "^;+ *inherits *:"))

((parameters
  (identifier) @constant.numeric)
  (#match? @constant.numeric "^[-+]?[0-9]+(.[0-9]+)?$"))

"_" @constant

[
  "@"
  "#"
] @punctuation.special

":" @punctuation.delimiter

[
  "["
  "]"
  "("
  ")"
] @punctuation.bracket

"." @operator

(predicate_type) @punctuation.special

(quantifier) @operator

(comment) @comment

(negated_field
  "!" @operator
  (identifier) @variable.other.member)

(field_definition
  name: (identifier) @variable.other.member)

(named_node
  name: (identifier) @variable)

(predicate
  name: (identifier) @function)

(anonymous_node
  (string) @string)

(capture
  (identifier) @type)

(escape_sequence) @constant.character.escape

(string) @string
