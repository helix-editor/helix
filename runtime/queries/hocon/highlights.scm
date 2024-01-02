(comment) @comment

(null) @constant.builtin
[(true) (false)] @constant.builtin.boolean
(number) @constant.numeric
(string) @string
(multiline_string) @string
(string (escape_sequence) @constant.character.escape)
(unquoted_string) @string

(value [":" "=" "+=" ] @operator)

(substitution (_) @string)
(substitution ["${" "${?" "}"] @punctuation.special)

[ 
  "url"
  "file"
  "classpath"
  "required"
] @function.builtin

(include "include" @include)

[ "(" ")" "[" "]" "{" "}" ]  @punctuation.bracket

(unit) @keyword
(path (_) @keyword)
(unquoted_path "." @punctuation.delimiter)
[ "," ] @punctuation.delimiter
