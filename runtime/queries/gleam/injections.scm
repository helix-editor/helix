((comment) @injection.content
 (#set! injection.language "comment"))

; Inject markdown into documentation comments
((doc_comment_content) @injection.content
 (#set! injection.language "markdown")
 (#set! injection.combined))
