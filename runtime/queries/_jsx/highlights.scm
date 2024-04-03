; Opening elements
; ----------------

(jsx_opening_element ((identifier) @constructor
 (#match? @constructor "^[A-Z]")))

(jsx_opening_element (identifier) @tag)

; Closing elements
; ----------------

(jsx_closing_element ((identifier) @constructor
 (#match? @constructor "^[A-Z]")))

(jsx_closing_element (identifier) @tag)

; Self-closing elements
; ---------------------

(jsx_self_closing_element ((identifier) @constructor
 (#match? @constructor "^[A-Z]")))

(jsx_self_closing_element (identifier) @tag)

; Attributes
; ----------

(jsx_attribute (property_identifier) @variable.other.member)

; Punctuation
; -----------

; Handle attribute delimiter (<Component color="red"/>)
(jsx_attribute "=" @punctuation.delimiter)

; <Component>
(jsx_opening_element ["<" ">"] @punctuation.bracket)

; </Component>
(jsx_closing_element ["</" ">"] @punctuation.bracket)

; <Component />
(jsx_self_closing_element ["<" "/>"] @punctuation.bracket)
