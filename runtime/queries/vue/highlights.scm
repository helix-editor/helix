(tag_name) @tag
(end_tag) @tag

(directive_name) @keyword
(directive_argument) @constant

(attribute
  (attribute_name) @attribute
  (quoted_attribute_value
    (attribute_value) @string)
)

(comment) @comment

[
  "<"
  ">"
  "</"
  "{{"
  "}}"
] @punctuation.bracket