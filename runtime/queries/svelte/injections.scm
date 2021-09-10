; injections.scm
; --------------
((style_element
  (raw_text) @css))

((attribute
   (attribute_name) @_attr
   (quoted_attribute_value (attribute_value) @css))
 (#eq? @_attr "style"))

((script_element
  (raw_text) @javascript))

((raw_text_expr) @javascript)

(
  (script_element
    (start_tag
      (attribute
        (quoted_attribute_value (attribute_value) @_lang)))
    (raw_text) @typescript)
  (#match? @_lang "(ts|typescript)")
)

(comment) @comment

