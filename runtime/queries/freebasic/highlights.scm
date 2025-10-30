; Keywords
"DIM" @keyword
"IF" @keyword
"THEN" @keyword
"ELSE" @keyword
"END IF" @keyword
"WHILE" @keyword
"WEND" @keyword
"FOR" @keyword
"TO" @keyword
"STEP" @keyword
"NEXT" @keyword
"DO" @keyword
"LOOP" @keyword
"UNTIL" @keyword
"SUB" @keyword
"END SUB" @keyword
"FUNCTION" @keyword
"END FUNCTION" @keyword
"RETURN" @keyword
"PRINT" @keyword
"INPUT" @keyword
"SLEEP" @keyword
"AS" @keyword
"AND" @keyword.operator
"OR" @keyword.operator
"NOT" @keyword.operator

; Types
(type_identifier) @type

; Functions
(function_declaration name: (identifier) @function)
(sub_declaration name: (identifier) @function)
(call_expression function: (identifier) @function.call)

; Literals
(number_literal) @number
(string_literal) @string

; Comments
(comment) @comment

; Operators
"=" @operator
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"^" @operator
"<>" @operator
"<" @operator
">" @operator
"<=" @operator
">=" @operator

; Punctuation
"(" @punctuation.bracket
")" @punctuation.bracket
"," @punctuation.delimiter
";" @punctuation.delimiter

; Variables (should be last to avoid conflicts)
(identifier) @variable
