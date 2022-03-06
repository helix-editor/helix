(((command) @__command__
  (message) @injection.content)
 (#match? @__command__ "^(x|exec)$")
 (#set! injection.language "bash"))
