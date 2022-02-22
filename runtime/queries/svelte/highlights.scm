; Special identifiers
;--------------------

; TODO:
((element (start_tag (tag_name) @_tag) (text) @markup.heading)
 (#match? @_tag "^(h[0-9]|title)$"))

((element (start_tag (tag_name) @_tag) (text) @markup.bold)
 (#match? @_tag "^(strong|b)$"))

((element (start_tag (tag_name) @_tag) (text) @markup.italic)
 (#match? @_tag "^(em|i)$"))

; ((element (start_tag (tag_name) @_tag) (text) @markup.strike)
; (#match? @_tag "^(s|del)$"))

((element (start_tag (tag_name) @_tag) (text) @markup.underline)
 (#eq? @_tag "u"))

((element (start_tag (tag_name) @_tag) (text) @markup.inline)
 (#match? @_tag "^(code|kbd)$"))

((element (start_tag (tag_name) @_tag) (text) @markup.link.url)
 (#eq? @_tag "a"))

((attribute
   (attribute_name) @_attr
   (quoted_attribute_value (attribute_value) @markup.link.url))
 (#match? @_attr "^(href|src)$"))

(tag_name) @tag
(attribute_name) @variable.other.member
(erroneous_end_tag_name) @error
(comment) @comment

[
  (attribute_value)
  (quoted_attribute_value)
] @string

[
  (text)
  (raw_text_expr)
] @none

[
  (special_block_keyword)
  (then)
  (as)
] @keyword

[
  "{"
  "}"
] @punctuation.brackets

"=" @operator

[
  "<"
  ">"
  "</"
  "/>"
  "#"
  ":"
  "/"
  "@"
] @punctuation.definition.tag
