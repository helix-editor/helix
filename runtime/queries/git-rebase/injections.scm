(((command) @_attribute
  (message)? @injection.content)
 (#match? @_attribute "^(x|exec)$")
 (#set! injection.language "bash")
)
