((comment (scissors))
 (message) @injection.content
 (#set! injection.include-children)
 (#set! injection.language "diff"))

((rebasecommand) @injection.content
 (#set! injection.include-children)
 (#set! injection.language "git-rebase"))
