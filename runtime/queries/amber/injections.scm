; Inject bash into command content
((command_content) @injection.content
 (#set! injection.language "bash"))

; Inject markdown into documentation comments (///)
((comment) @injection.content
 (#match? @injection.content "^///")
 (#set! injection.language "markdown")
 (#set! injection.combined))

; Regular comments (excluding doc comments)
((comment) @injection.content
 (#not-match? @injection.content "^///")
 (#set! injection.language "comment"))
