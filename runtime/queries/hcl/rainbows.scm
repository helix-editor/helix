; HCL splits its delimiters into start/end nodes (block_start `{` / block_end
; `}`, object_start/end, tuple_start/end), so the brackets aren't direct
; children of the block/object/tuple that nests — scope those wrapping nodes
; with rainbow.include-children so the nested delimiters still highlight.
((block) @rainbow.scope (#set! rainbow.include-children))
((object) @rainbow.scope (#set! rainbow.include-children))
((tuple) @rainbow.scope (#set! rainbow.include-children))

; A function call's parens are direct children, so no include-children needed.
(function_call) @rainbow.scope

[
  "{" "}"
  "[" "]"
  "(" ")"
] @rainbow.bracket
