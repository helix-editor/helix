(comment) @comment

(headword
  (word) @label)

(attribute
  name: (word) @variable.other.member)

"=" @operator

(string) @string

((value) @constant.numeric.integer
  (#match? @constant.numeric.integer "^-?[0-9]+([.][0-9]+)?$"))

((value) @string.special
  (#not-match? @string.special "^-?[0-9]+([.][0-9]+)?$"))

(list
  (item) @string.special)

[
  "["
  "]"
] @punctuation.bracket

"," @punctuation.delimiter
