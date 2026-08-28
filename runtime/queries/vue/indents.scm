; Vue single-file components are HTML-like: indent the contents of <template>
; and of nested elements, and pull the end tag — and the close of a multi-line
; start / self-closing tag — back to the element's column. Mirrors the html/jsx
; rules.
;
; <script> and <style> bodies are deliberately *not* indented: Vue convention
; (and Prettier's default) keeps them at column 0, and their embedded JS/CSS is
; indented by the injected language's own indent query.
[
  (element)
  (template_element)
] @indent

(end_tag) @outdent
(start_tag ">" @outdent)
(self_closing_tag "/>" @outdent)
