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

((value) @constant.numeric.float
  (#match? @constant.numeric.float "^[+-]?(([0-9]*\.[0-9]+([eE][+-]?[0-9]+)?)|([0-9]+[eE][+-]?[0-9]+))$"))

((value) @constant.numeric.integer
  ; according to the Java spec, hex literals must represent a 64bit int;
  ; overflow (too long) is considered an error.
  ; however, since these are just strings,
  ; a long hex-literal could represent a BigInt, so we allow it
  (#match? @constant.numeric.integer "^([+-]?[0-9]+)|(0[xX][0-9a-fA-F]+)$"))

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
