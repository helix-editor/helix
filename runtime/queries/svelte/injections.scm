; injections.scm
; --------------
((style_element
  (raw_text) @injection.content)
  (#set! injection.language "css"))

((attribute
   (attribute_name) @_attr
   (quoted_attribute_value (attribute_value) @css))
 (#eq? @_attr "style"))

((script_element
  (raw_text) @injection.content)
  (#set! injection.language "javascript"))

((raw_text_expr) @injection.content
 (#set! injection.language "javascript"))

(
  (script_element
    (start_tag
      (attribute
        (quoted_attribute_value (attribute_value) @_lang)))
    (raw_text) @injection.content)
  (#match? @_lang "(ts|typescript)")
  (#set! injection.language "typescript")
)

((comment) @injection.content
 (#set! injection.language "comment"))
