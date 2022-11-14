((comment) @injection.content
 (#set! injection.language "jsdoc")
 (#match? @injection.content "^/\\*+"))

((comment) @injection.comment
 (#set! injection.language "comment")
 (#match? @injection.content "^//"))
