(comment) @comment

[
  (title)
] @markup.heading.1

[
  "adornment"
] @markup.heading.marker

[
  (target)
  (reference)
] @markup.link.url

[
  "bullet"
] @markup.list.unnumbered

(strong) @markup.bold
(emphasis) @markup.italic
(literal) @markup.raw.inline

(list_item
  (term) @markup.bold
  (classifier)? @markup.italic)

(directive
  [".." (type) "::"] @function
)

(field
  [":" (field_name) ":"] @variable.other.member
)

(interpreted_text) @markup.raw.inline

(interpreted_text (role)) @keyword
