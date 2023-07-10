; Properties
;-----------

(table [
  (bare_key)
  (dotted_key)
  (quoted_key)
] @type)

(table_array_element [
  (bare_key)
  (dotted_key)
  (quoted_key)
] @type)

(pair [
  (bare_key)
  (dotted_key)
  (quoted_key)
] @variable.other.member)

; Literals
;---------

(boolean) @constant.builtin.boolean
(comment) @comment
(string) @string
(integer) @constant.numeric.integer
(float) @constant.numeric.float
(offset_date_time) @string.special
(local_date_time) @string.special
(local_date) @string.special
(local_time) @string.special

; Punctuation
;------------

"." @punctuation.delimiter
"," @punctuation.delimiter

"=" @operator

"[" @punctuation.bracket
"]" @punctuation.bracket
"[[" @punctuation.bracket
"]]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
