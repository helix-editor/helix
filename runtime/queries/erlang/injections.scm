((line_comment (comment_content) @injection.content)
 (#set! injection.language "edoc")
 (#set! injection.include-children)
 (#set! injection.combined))

((comment (comment_content) @injection.content)
 (#set! injection.language "comment"))

; EEP-59 doc attributes use markdown by default.
(attribute
  name: (atom) @_attribute
  (arguments [
    (string (quoted_content) @injection.content)
    (sigil (quoted_content) @injection.content)
  ])
 (#set! injection.language "markdown")
 (#any-of? @_attribute "doc" "moduledoc"))
