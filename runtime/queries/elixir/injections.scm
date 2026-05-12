; Elixir Code Comments
((comment) @injection.content
 (#set! injection.language "comment"))

; Elixir Regular Expressions
((sigil
  (sigil_name) @_sigil_name
  (quoted_content) @injection.content)
 (#match? @_sigil_name "^(R|r)$")
 (#set! injection.language "regex")
 (#set! injection.combined))

; Elixir Markdown Documentation
(unary_operator
  operator: "@"
  operand: (call
  target: ((identifier) @_identifier (#match? @_identifier "^(module|type|short)?doc$"))
    (arguments [
      (string (quoted_content) @injection.content)
      (sigil (quoted_content) @injection.content)
  ])) (#set! injection.language "markdown"))

; Zigler Sigils
((sigil
  (sigil_name) @_sigil_name
  (quoted_content) @injection.content)
 (#match? @_sigil_name "^(Z|z)$")
 (#set! injection.language "zig")
 (#set! injection.combined))

; Jason Sigils
((sigil
  (sigil_name) @_sigil_name
  (quoted_content) @injection.content)
 (#match? @_sigil_name "^(J|j)$")
 (#set! injection.language "json")
 (#set! injection.combined))

; Phoenix Live View HEEx Sigils
((sigil
  (sigil_name) @_sigil_name
  (quoted_content) @injection.content)
 (#eq? @_sigil_name "H")
 (#set! injection.language "heex")
 (#set! injection.combined))

; Phoenix Live View Component Macros
(call 
  (identifier) @_identifier
  (arguments
    (atom)+
    (keywords (pair 
      ((keyword) @_keyword (#eq? @_keyword "doc: "))
      [
        (string (quoted_content) @injection.content)
        (sigil (quoted_content) @injection.content)
      ]))
  (#match? @_identifier "^(attr|slot)$")
  (#set! injection.language "markdown")))
