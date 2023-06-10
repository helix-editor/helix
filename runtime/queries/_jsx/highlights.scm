; Opening elements
; ----------------

(jsx_opening_element ((identifier) @constructor
 (#match? @constructor "^[A-Z]")))

; Handle the dot operator effectively - <My.Component>
(jsx_opening_element ((nested_identifier (identifier) @tag (identifier) @constructor)))

(jsx_opening_element (identifier) @tag)

; Closing elements
; ----------------

(jsx_closing_element ((identifier) @constructor
 (#match? @constructor "^[A-Z]")))

; Handle the dot operator effectively - </My.Component>
(jsx_closing_element ((nested_identifier (identifier) @tag (identifier) @constructor)))

(jsx_closing_element (identifier) @tag)

; Self-closing elements
; ---------------------

(jsx_self_closing_element ((identifier) @constructor
 (#match? @constructor "^[A-Z]")))

; Handle the dot operator effectively - <My.Component />
(jsx_self_closing_element ((nested_identifier (identifier) @tag (identifier) @constructor)))

(jsx_self_closing_element (identifier) @tag)

; Attributes
; ----------

(jsx_attribute (property_identifier) @variable.other.member)

; Punctuation
; -----------

; Handle attribute delimiter
(jsx_attribute "=" @punctuation.delimiter)

; <Component>
(jsx_opening_element ["<" ">"] @punctuation.bracket)

; </Component>
(jsx_closing_element ["<" "/" ">"] @punctuation.bracket)

; <Component />
(jsx_self_closing_element ["<" "/" ">"] @punctuation.bracket)

; <> ... </>
(jsx_fragment ["<" "/" ">"] @punctuation.bracket)
