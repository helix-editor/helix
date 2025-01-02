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

":" @punctuation.delimiter

[
  "["
  "]"
  "("
  ")"
] @punctuation.bracket

"." @operator

(quantifier) @operator

(comment) @comment

(negated_field
  "!" @operator
  (identifier) @variable.other.member)

(field_definition
  name: (identifier) @variable.other.member)

(named_node
  name: (identifier) @tag)

((predicate
   "#" @function.builtin
   name: (identifier) @function.builtin @_name
   type: (predicate_type) @function.builtin)
 (#any-of? @_name "eq" "match" "any-of" "not-any-of" "is" "is-not" "not-same-line" "not-kind-eq" "set" "select-adjacent" "strip"))
(predicate name: (identifier) @error)

(capture) @label

(escape_sequence) @constant.character.escape

(string) @string
