; group
; target
; function
; variable


(block
  (identifier) @_block (#eq? @_block "group")
  (string_lit
    (template_literal) @name)) @definition.module

(block
  (identifier) @_block (#eq? @_block "target")
  (string_lit
    (template_literal) @name)) @definition.struct

(block
  (identifier) @_block (#eq? @_block "function")
  (string_lit
    (template_literal) @name)) @definition.function

(block
  (identifier) @_block (#eq? @_block "variable")
  (string_lit
    (template_literal) @name)) @definition.constant


; (config_file
;   (body
;   (block
;     (identifier) @_block (#eq? @_block "function")
;     (string_lit
;       (template_literal) @name)) @definition.function))
