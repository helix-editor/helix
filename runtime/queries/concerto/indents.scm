; Concerto Language - Indent Queries (Helix)
; ============================================
; Helix-specific indentation rules. For use in helix-editor/helix at
; runtime/queries/concerto/indents.scm
;
; Helix uses @indent and @outdent captures, same as tree-sitter convention.
; See: https://docs.helix-editor.com/guides/indent.html

; Indent inside declaration bodies and decorator argument lists
[
  (class_body)
  (enum_body)
  (map_body)
  (decorator_arguments)
] @indent

; Outdent at closing braces and parentheses
[
  "}"
  ")"
] @outdent
