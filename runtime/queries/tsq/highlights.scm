((predicate
  name: (identifier) @_name
  parameters:
    (parameters
      (string
        "\"" @string
        "\"" @string) @string.regexp
      .
      (string) .))
  (#any-of? @_name "gsub" "not-gsub"))

((comment) @keyword.directive
  (#match? @keyword.directive "^;+\s*format\-ignore\s*$"))

((program
  .
  (comment)*
  .
  (comment) @keyword.directive)
  (#match? @keyword.directive "^;+ *extends *$"))

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
  (identifier) @property)

(field_definition
  name: (identifier) @property)

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
