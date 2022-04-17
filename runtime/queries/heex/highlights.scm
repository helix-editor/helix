; https://github.com/connorlay/tree-sitter-heex/blob/592e22292a367312c35e13de7fdb888f029981d6/queries/highlights.scm
; HEEx delimiters
[
  "<!"
  "<!--"
  "<"
  "<%!--"
  "<%#"
  ">"
  "</"
  "--%>"
  "-->"
  "/>"
  ; These could be `@keyword`s but the closing `>` wouldn't be highlighted
  ; as `@keyword`
  "<:"
  "</:"
] @punctuation.bracket

; Non-comment or tag delimiters
[
  "{"
  "}"
  "<%"
  "<%="
  "<%%="
  "%>"
] @keyword

; HEEx operators are highlighted as such
"=" @operator

; HEEx inherits the DOCTYPE tag from HTML
(doctype) @constant

; HEEx comments are highlighted as such
(comment) @comment

; HEEx tags are highlighted as HTML
(tag_name) @tag

; HEEx slots are highlighted as atoms (symbols)
(slot_name) @string.special.symbol

; HEEx attributes are highlighted as HTML attributes
(attribute_name) @attribute
[
  (attribute_value)
  (quoted_attribute_value)
] @string

; HEEx components are highlighted as Elixir modules and functions
(component_name
  [
    (module) @module
    (function) @function
    "." @punctuation.delimiter
  ])
