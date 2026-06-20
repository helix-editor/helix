; See: https://docs.helix-editor.com/guides/indent.html

; indent
; ------

[
  ; <..., ...>
  (formals)
  (params)

  ; (...| ...)
  (alt)
] @indent

; outdent
; -------

[
  "}"
  ")"
  ">"
] @outdent

; align
; -----

; | ... | ...
(rule_body
  . (top_level_term) @anchor
  ) @align

; N/A or unused:
; --------------
; indent.always
; outdent.always
; extend
; extend.prevent-once
