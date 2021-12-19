; TODO: re-add when markdown is added.
; ((triple_string) @injection.content
;  (#offset! @injection.content 0 3 0 -3)
;  (#set! injection.language "markdown"))

((comment) @injection.content
 (#set! injection.language "comment"))
