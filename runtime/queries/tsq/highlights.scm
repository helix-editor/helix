; mark the string passed #match? as a regex
(((predicate_name) @function
  (capture)
  (string) @string.regexp)
 (#eq? @function "#match?"))

; highlight inheritance comments
(((comment) @keyword.directive)
 (#match? @keyword.directive "^; +inherits *:"))

[
  "("
  ")"
  "["
  "]"
] @punctuation.bracket

":" @punctuation.delimiter
"!" @operator

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

((predicate_name) @function
 (#any-of? @function "#eq?" "#match?" "#any-of?" "#not-any-of?" "#is?" "#is-not?" "#not-same-line?" "#not-kind-eq?" "#set!" "#select-adjacent!" "#strip!"))
(predicate_name) @error

(escape_sequence) @constant.character.escape

(node_name) @tag
(variable) @variable
