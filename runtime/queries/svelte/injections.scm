; inherits html
((style_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_lang)))
  (raw_text) @injection.content)
  (#eq? @_attr "lang")
  (#any-of? @_lang "scss" "postcss" "less")
  (#set! injection.language "scss"))

((svelte_raw_text) @injection.content
  (#set! injection.language "typescript"))

((script_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_lang)))
  (raw_text) @injection.content)
  (#eq? @_attr "lang")
  (#any-of? @_lang "ts" "typescript")
  (#set! injection.language "typescript"))

((script_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_lang)))
  (raw_text) @injection.content)
  (#eq? @_attr "lang")
  (#any-of? @_lang "js" "javascript")
  (#set! injection.language "javascript"))

((element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @injection.language)))
  (text) @injection.content)
  (#eq? @_attr "lang")
  (#eq? @injection.language "pug"))
