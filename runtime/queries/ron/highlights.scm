; Structs
;------------

(enum_variant) @type.enum.variant
(struct_entry (_) @variable.other.member ":")
(struct_name (identifier)) @type
(unit_struct) @type.builtin

; Literals
;------------

(string) @string
(boolean) @constant.builtin.boolean
(integer) @constant.numeric.integer
(float) @constant.numeric.float
(char) @constant.character

; Comments
;------------

(line_comment) @comment.line
(block_comment) @comment.block


; Punctuation
;------------

"," @punctuation.delimiter
":" @punctuation.delimiter

"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket

"-" @operator

; Special
;------------
(escape_sequence) @constant.character.escape
