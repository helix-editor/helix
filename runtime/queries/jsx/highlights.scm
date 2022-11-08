; Highlight component names differently
(jsx_opening_element ((identifier) @constructor
 (#match? @constructor "^[A-Z]")))

; Handle the dot operator effectively - <My.Component>
(jsx_opening_element ((nested_identifier (identifier) @tag (identifier) @constructor)))

; Highlight brackets differently
(jsx_opening_element ["<" ">"] @punctuation.bracket)

(jsx_closing_element ((identifier) @constructor
 (#match? @constructor "^[A-Z]")))

; Handle the dot operator effectively - </My.Component>
(jsx_closing_element ((nested_identifier (identifier) @tag (identifier) @constructor)))

; Highlight brackets differently
(jsx_closing_element ["<" "/" ">"] @punctuation.bracket)

(jsx_self_closing_element ((identifier) @constructor
 (#match? @constructor "^[A-Z]")))

; Handle the dot operator effectively - <My.Component />
(jsx_self_closing_element ((nested_identifier (identifier) @tag (identifier) @constructor)))

; Highlight brackets differently
(jsx_self_closing_element ["<" "/" ">"] @punctuation.bracket)

; Handle attribute delimiter
(jsx_attribute "=" @punctuation.delimiter)

(jsx_opening_element (identifier) @tag)
(jsx_closing_element (identifier) @tag)
(jsx_self_closing_element (identifier) @tag)
(jsx_attribute (property_identifier) @variable.other.member)

; inherits: ecma
