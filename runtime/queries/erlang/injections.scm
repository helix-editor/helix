((line_comment (comment_content) @injection.content)
 (#set! injection.language "edoc")
 (#set! injection.include-children)
 (#set! injection.combined))

((comment (comment_content) @injection.content)
 (#set! injection.language "comment"))
