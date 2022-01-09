[
  "for"
] @keyword

(attribute (identifier) @property)
(object_elem (identifier) @property)

(block (identifier) @type)
(one_line_block (identifier) @type)

(function_call (identifier) @function)

[
  (string_literal)
  (quoted_template)
  (heredoc)
] @string

(numeric_literal) @number

[
  (true)
  (false)
  (null)
] @constant.builtin

(comment) @comment

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket
