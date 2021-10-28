; Properties
;-----------

(bare_key) @variable.other.member
(quoted_key) @string

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
