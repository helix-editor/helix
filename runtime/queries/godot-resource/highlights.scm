(section (identifier) @type.builtin)

(attribute (identifier) @attribute)
(property (path) @attribute)
(constructor (identifier) @constructor)

(string) @string
(integer) @constant.numeric.integer
(float) @constant.numberic.float

(true) @constant.builtin.boolean
(false) @constant.builtin.boolean

[
  "["
  "]"
] @tag

[
  "("
  ")"
  "{"
  "}"
] @punctuation.bracket

"=" @operator

(ERROR) @error