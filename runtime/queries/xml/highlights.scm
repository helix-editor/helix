(comment) @comment

[
    "DOCTYPE"
    "ELEMENT"
    "ATTLIST"
] @keyword

[
    "#REQUIRED"
    "#IMPLIED"
    "#FIXED"
    "#PCDATA"
] @keyword.directive

[
    "EMPTY"
    "ANY"
    "SYSTEM"
    "PUBLIC"
] @constant

(doctype) @variable
(element_name) @variable

"xml" @tag
(tag_name) @tag

[
    "encoding"
    "version"
    "standalone"
] @attribute
(attribute_name) @attribute

(system_literal) @string
(pubid_literal) @string
(attribute_value) @string

[
    "<" ">" "</" "/>" "<?" "?>" "<!"
] @punctuation.bracket
