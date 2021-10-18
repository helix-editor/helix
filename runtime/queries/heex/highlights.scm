; adapted from https://github.com/nvim-treesitter/nvim-treesitter/blob/58dd95f4a4db38a011c8f28564786c9d98b010c8/queries/heex/highlights.scm

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
