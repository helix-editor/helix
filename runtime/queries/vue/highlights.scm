(tag_name) @tag
(end_tag) @tag

(directive_name) @keyword
(directive_argument) @constant

(attribute
  (attribute_name) @attribute
  [(attribute_value) (quoted_attribute_value)]? @string)
 
(directive_attribute
  (directive_name) @attribute
  (directive_argument)? @attribute
  (directive_modifiers)? @attribute
  [(attribute_value) (quoted_attribute_value)]? @string) 

(comment) @comment

[
  "<"
  ">"
  "</"
  "{{"
  "}}"
  "/>" 
] @punctuation.bracket
"=" @punctuation.delimiter

