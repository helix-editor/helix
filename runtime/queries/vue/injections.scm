(directive_attribute
  (directive_name) @keyword
  (quoted_attribute_value
    (attribute_value) @injection.content)
 (#set! injection.language "javascript"))

((interpolation
  (raw_text) @injection.content)
 (#set! injection.language "javascript"))

((script_element
  (raw_text) @injection.content)
 (#set! injection.language "javascript"))

((style_element
  (raw_text) @injection.content)
 (#set! injection.language "css"))

((comment) @injection.content
 (#set! injection.language "comment"))
