; once a diff grammar is available, we can inject diff highlighting into the
; trailer after scissors (git commit --verbose)
; see https://github.com/helix-editor/helix/pull/1338#issuecomment-1000013539
;
; ((comment (scissors))
;  (message) @injection.content
;  (#set! injection.language "diff"))

; ---

; once a rebase grammar is available, we can inject rebase highlighting into
; interactive rebase summary sections like so:
;
; ((rebase_command) @injection.content
;  (#set! injection.language "git-rebase"))
