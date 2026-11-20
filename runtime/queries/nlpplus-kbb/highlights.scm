(comment) @comment

; A concept at the left margin, then concepts nested under it.
(entry
  concept: (concept) @namespace)

(nested_entry
  concept: (concept) @type)

(attribute
  name: (_) @variable.other.member)

[
  ":"
  "="
] @operator

[
  "["
  "]"
] @punctuation.bracket

"," @punctuation.delimiter

((value) @constant.numeric.integer
  (#match? @constant.numeric.integer "^-?[0-9]+(\\.[0-9]+)?$"))

((value) @string.special
  (#not-match? @string.special "^-?[0-9]+(\\.[0-9]+)?$"))

(item) @string.special

(list
  (string) @string)

(attribute
  value: (string) @string)
