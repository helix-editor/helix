; mark the string passed #match? as a regex
(((predicate_name) @function
  (capture)
  (string) @string.regexp)
 (#eq? @function "#match?"))

; highlight inheritance comments
((query . (comment) @keyword.directive)
 (#match? @keyword.directive "^;\ +inherits *:"))

[
  "("
  ")"
  "["
  "]"
] @punctuation.bracket

":" @punctuation.delimiter

[
  (one_or_more)
  (zero_or_one)
  (zero_or_more)
] @operator

[
  (wildcard_node)
  (anchor)
] @constant.builtin

[
  (anonymous_leaf)
  (string)
] @string

(comment) @comment

(field_name) @variable.other.member

(capture) @label

(predicate_name) @function

(escape_sequence) @constant.character.escape

(node_name) @variable
