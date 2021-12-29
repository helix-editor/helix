((comment (scissors))
 (message) @injection.content
 (#set! injection.include-children)
 (#set! injection.language "diff"))

; once a rebase grammar is available, we can inject rebase highlighting into
; interactive rebase summary sections like so:
;
; ((rebase_command) @injection.content
;  (#set! injection.language "git-rebase"))
