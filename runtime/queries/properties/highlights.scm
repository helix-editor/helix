(comment) @comment

(key) @attribute

(value) @string

(value (escape) @constant.character.escape)

((index) @constant.numeric.integer
  (#match? @constant.numeric.integer "^[0-9]+$"))

((substitution (key) @constant)
  (#match? @constant "^[A-Z0-9_]+"))

((value) @constant.builtin.boolean
  (#any-of? @constant.builtin.boolean "true" "false" "enabled" "disabled"))

((value) @constant.numeric.integer
  (#match? @constant.numeric.integer "^-?[0-9]+$"))

((value) @constant.numeric.float
  (#match? @constant.numeric.float "^-?[0-9]+\.[0-9]$"))

((value) @string.special.path
  (#match? @string.special.path "^(\.{1,2})?/"))

(substitution
  (key) @function
  "::" @punctuation.special
  (secret) @string.special.symbol)

(property [ "=" ":" ] @keyword.operator)

[ "${" "}" ] @punctuation.special

(substitution ":" @punctuation.special)

[ "[" "]" ] @punctuation.bracket

[ "." "\\" ] @punctuation.delimiter
