(tag_name) @tag
(end_tag) @tag

(directive_name) @keyword
(directive_argument) @constant

(attribute
  (attribute_name) @attribute
  (quoted_attribute_value
    (attribute_value) @string)?
)

 (attribute
  (attribute_name) @attribute
)

 (attribute
   (attribute_name) @attribute
   "=" @attribute_name
   (#eq? @attribute_name "=")
) @attribute

 (directive_attribute
  (directive_name) @keyword
  "=" @attribute_name
  (#eq? @attribute_name "=")
 ) @attribute.empty

(comment) @comment

[
  "<"
  ">"
  "</"
  "{{"
  "}}"
  "/>" 
] @punctuation.bracket

