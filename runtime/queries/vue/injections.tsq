(directive_attribute
  (directive_name) @keyword
  (quoted_attribute_value
    (attribute_value) @injection.content)
 (#set! injection.language "javascript"))

((interpolation
  (raw_text) @injection.content)
 (#set! injection.language "javascript"))

; <script>
((script_element
    (start_tag) @_no_lang
    (raw_text) @injection.content)
  (#not-match? @_no_lang "lang=")
  (#set! injection.language "javascript"))

; <script lang="...">
((script_element
  (start_tag
    (attribute
    (attribute_name) @_attr_name
    (quoted_attribute_value (attribute_value) @injection.language)))
  (raw_text) @injection.content)
  (#eq? @_attr_name "lang"))

; <style>
((style_element
    (start_tag) @_no_lang
    (raw_text) @injection.content)
  (#not-match? @_no_lang "lang=")
  (#set! injection.language "css"))

; <style lang="...">
((style_element
  (start_tag
    (attribute
      (attribute_name) @_attr_name
      (quoted_attribute_value (attribute_value) @injection.language)))
   (raw_text) @injection.content)
 (#eq? @_attr_name "lang"))

((comment) @injection.content
 (#set! injection.language "comment"))
