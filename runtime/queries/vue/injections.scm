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

; <script lang="js|javascript">
((script_element
    (start_tag (attribute (quoted_attribute_value (attribute_value) @_lang)))
    (raw_text) @injection.content)
    (#match? @_lang "^(js|javascript)$")
    (#set! injection.language "javascript"))

; <script lang="ts|typescript">
((script_element
    (start_tag (attribute (quoted_attribute_value (attribute_value) @_lang)))
    (raw_text) @injection.content)
    (#match? @_lang "^(ts|typescript)$")
    (#set! injection.language "typescript"))

; <style>
((style_element
    (start_tag) @_no_lang
    (raw_text) @injection.content)
  (#not-match? @_no_lang "lang=")
  (#set! injection.language "css"))

; <style lang="...">
((style_element
    (start_tag (attribute (quoted_attribute_value (attribute_value) @injection.language)))
    (raw_text) @injection.content)
    (#match? @injection.language "^(css|scss)$"))

((comment) @injection.content
 (#set! injection.language "comment"))
