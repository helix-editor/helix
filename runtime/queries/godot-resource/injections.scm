((comment) @injection.content
 (#set! injection.language "comment"))

; ((section) @injection.content
;  (#set! injection.language "comment"))

((section 
  (attribute 
    (identifier) @_type
    (string) @_is_shader)
  (property 
    (path) @_is_code
    (string) @injection.content))
  (#match? @_type "type")
  (#match? @_is_shader "Shader")
  (#eq? @_is_code "code")
  (#set! injection.language "glsl")
)
