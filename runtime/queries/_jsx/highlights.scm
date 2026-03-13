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

; Attributes
; ----------

(jsx_attribute (property_identifier) @attribute)

; Opening elements
; ----------------

(jsx_opening_element (identifier) @tag)

(jsx_opening_element ((identifier) @constructor
 (#match? @constructor "^[A-Z]")))

; Closing elements
; ----------------

(jsx_closing_element (identifier) @tag)

(jsx_closing_element ((identifier) @constructor
 (#match? @constructor "^[A-Z]")))

; Self-closing elements
; ---------------------

(jsx_self_closing_element (identifier) @tag)

(jsx_self_closing_element ((identifier) @constructor
 (#match? @constructor "^[A-Z]")))
