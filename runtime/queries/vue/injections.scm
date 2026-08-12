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

; NOTE: <template> content is intentionally *not* injected. The tree-sitter-vue
; grammar parses the template body natively as an HTML-like element tree
; (`element`/`start_tag`/`directive_attribute`/`interpolation`/…), so it is
; highlighted by vue's own `highlights.scm` — which understands Vue directives
; (`v-if`, `:prop`, `@click`) that a plain HTML injection would not. A former
; `(template_element (text) @injection.content) html` rule injected only the
; loose inter-tag text fragments (never the tags), adding no highlighting while
; breaking indentation: a caret at a `<template>`/element boundary landed in the
; whitespace-only HTML sub-layer, so indent-on-type computed column 0. The
; grammar parses the body as HTML elements regardless of any `lang="…"`
; attribute, so an alternate template language (e.g. pug) cannot be injected over
; it cleanly either.

((comment) @injection.content
 (#set! injection.language "comment"))
