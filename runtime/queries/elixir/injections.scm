((comment) @injection.content
 (#set! injection.language "comment"))

((sigil
  (sigil_name) @_sigil_name
  (quoted_content) @injection.content)
 (#match? @_sigil_name "^(r|R)$")
 (#set! injection.language "regex")
 (#set! injection.combined))

((sigil
  (sigil_name) @_sigil_name
  (quoted_content) @injection.content)
 (#eq? @_sigil_name "H")
 (#set! injection.language "heex")
 (#set! injection.combined))

(unary_operator
  operator: "@"
  operand: (call
  target: ((identifier) @_identifier (#match? @_identifier "^(module|type|short)?doc$"))
    (arguments [
      (string (quoted_content) @injection.content)
      (sigil (quoted_content) @injection.content)
  ])) (#set! injection.language "markdown"))
