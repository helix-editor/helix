[
  (jsx_element)
  (jsx_self_closing_element)
] @indent

(parenthesized_expression) @indent

; (jsx_element)/(jsx_self_closing_element) indent everything after the opening
; `<tag`, including the line that closes the tag. Pull those closing tokens back
; to the tag's own column (as ecma does for `}`/`)`/`]`): the `</tag>` of an
; element, the `/>` of a self-closing tag, and the `>` of an opening tag whose
; attributes wrap onto their own lines. Each only fires when the token begins a
; line, so single-line tags are unaffected.
(jsx_closing_element) @outdent
(jsx_self_closing_element "/>" @outdent)
(jsx_opening_element ">" @outdent)
