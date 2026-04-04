((diff) @injection.content
 (#set! injection.language "diff"))

((rebase_command) @injection.content
 (#set! injection.include-children)
 (#set! injection.language "git-rebase"))
