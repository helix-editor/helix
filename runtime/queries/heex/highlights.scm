(text) @text
(comment) @comment
(doctype) @constant

; HEEx attributes are highlighted as HTML attributes
(attribute_name) @attribute
(quoted_attribute_value) @string

[
  "%>"
  "/>"
  "<!"
  "<"
  "<%"
  "<%#"
  "<%%="
  "<%="
  "</"
  ">"
  "{"
  "}"
] @punctuation.bracket

[
  "="
] @operator

; HEEx tags are highlighted as HTML
(tag_name) @tag

; HEEx components are highlighted as types (Elixir modules)
(component_name) @type
