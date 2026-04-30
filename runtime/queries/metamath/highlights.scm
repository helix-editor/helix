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
(floating_stmt (label) @function)
(essential_stmt (label) @function)
(axiom_stmt (label) @function.definition)
(provable_stmt (label) @function.definition)

; Types
(typecode) @type

; Variables and constants in declarations
(constant_stmt (constant) @constant)
(variable_stmt (variable) @variable)

; Math symbols
(mathsymbol) @variable

; Proofs
(uncompressed_proof (label) @function.call)
(compressed_proof (label) @function.call)
(compressed_proof_block) @string

; Comments
(comment) @comment.block

; Parentheses in math expressions
"(" @punctuation.bracket
")" @punctuation.bracket

; File includes
(filename) @string.special.path
