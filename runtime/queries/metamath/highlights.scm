; Keywords and delimiters
[ "$c" "$v" "$d" "$f" "$e" "$a" "$p" "$=" ] @keyword
[ "${" "$}" ] @punctuation.bracket
[ "$[" "$]" ] @keyword.import
"$." @punctuation.delimiter

; Markup (update grammar.js to support)
; "####" @markup.heading.1
; "#*#*" @markup.heading.2
; "=-=-" @markup.heading.3
; "-.-." @markup.heading.4

; Builtin typecodes
[ "|-" "wff" "setvar" "class" ] @type.builtin

; Labels
(floatingstmt (label) @function)
(essentialstmt (label) @function)
(axiomstmt (label) @function.definition)
(provablestmt (label) @function.definition)

; Types
(typecode) @type

; Variables and constants in declarations
(constantstmt (constant) @constant)
(variablestmt (variable) @variable)

; Math symbols
(mathsymbol) @variable

; Proofs
(uncompressedproof (label) @function.call)
(compressedproof (label) @function.call)
(compressedproofblock) @string

; Comments
(comment) @comment.block

; Parentheses in math expressions
"(" @punctuation.bracket
")" @punctuation.bracket

; File includes
(filename) @string.special.path
