; Properties
;-----------

(bare_key) @property
(quoted_key) @string

; Literals
;---------

(boolean) @constant.builtin
(comment) @comment
(string) @string
(integer) @number
(float) @number
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
