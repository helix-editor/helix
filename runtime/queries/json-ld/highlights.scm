; inherits: json

; https://www.w3.org/TR/json-ld/#syntax-tokens-and-keywords
((string (string_content) @keyword)
 (#any-of? @keyword
   "@base"
   "@container"
   "@context"
   "@direction"
   "@graph"
   "@id"
   "@import"
   "@included"
   "@index"
   "@json"
   "@language"
   "@list"
   "@nest"
   "@none"
   "@prefix"
   "@propagate"
   "@protected"
   "@reverse"
   "@set"
   "@type"
   "@value"
   "@version"
   "@vocab"))

((pair
  value: (string (string_content) @string.special.url))
 (#match? @string.special.url "^https?://"))

((array
  (string (string_content) @string.special.url))
  (#match? @string.special.url "^https?://"))

; https://www.w3.org/TR/json-ld/#dfn-base-direction
((pair
  key: (string (string_content) @keyword)
  value: (string (string_content) @type.enum.variant))
 (#eq? @keyword "@direction")
 (#any-of? @type.enum.variant "ltr" "rtl"))
