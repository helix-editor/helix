(tag_name) @tag
(erroneous_end_tag_name) @error
(doctype) @constant
(attribute_name) @attribute
(comment) @comment

((attribute
  (attribute_name) @_attr
  (quoted_attribute_value (attribute_value) @markup.link.url))
 (#any-of? @_attr "href" "src"))

((element
  (start_tag
    (tag_name) @_tag)
  (text) @markup.link.label)
  (#eq? @_tag "a"))

(attribute [(attribute_value) (quoted_attribute_value)] @string)

((element
  (start_tag
    (tag_name) @_tag)
  (text) @markup.bold)
  (#any-of? @_tag "strong" "b"))

((element
  (start_tag
    (tag_name) @_tag)
  (text) @markup.italic)
  (#any-of? @_tag "em" "i"))

((element
  (start_tag
    (tag_name) @_tag)
  (text) @markup.strikethrough)
  (#any-of? @_tag "s" "del"))

[
  "<"
  ">"
  "</"
  "/>"
  "<!"
] @punctuation.bracket

"=" @punctuation.delimiter
