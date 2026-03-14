; Inject CSS into style elements
(style_element
  (raw_text) @injection.content
  (#set! injection.language "css"))

; Inject JavaScript/TypeScript into server blocks
; Note: statement is inlined, so we need to match specific statement types
; Commenting out for now as it requires matching all concrete statement types

; Template string interpolations
(template_substitution
  (expression) @injection.content
  (#set! injection.language "typescript"))
